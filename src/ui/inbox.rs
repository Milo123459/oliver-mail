use std::ops::Range;

use crate::app::{AppState, SidebarEmail};
use crate::models::{
    Email, GoogleAccount, TempEmail, Theme, get_gmail_mail, get_gmail_message, get_mail,
    refresh_token,
};
use crate::ui::EmailView;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use futures_util::StreamExt;
use gpui::{
    AnyElement, Context, Entity, Render, Task, UniformListScrollHandle, Window, div, prelude::*,
    px, rgb, uniform_list,
};
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::mpsc;

enum Account {
    Temp(TempEmail),
    Google(GoogleAccount),
}

impl Account {
    fn key(&self) -> String {
        match self {
            Account::Temp(account) => format!("temp:{}", account.id),
            Account::Google(account) => format!("google:{}", account.email),
        }
    }
}

/// Messages from the temp-mail listener (running on tokio) to the UI.
enum TempMailEvent {
    Token(String),
    Emails(Vec<Email>),
    Failed,
}

pub struct Inbox {
    pub emails: Vec<Email>,
    pub loading: bool,
    pub state: Entity<AppState>,
    pub theme: Entity<Theme>,
    pub email_view: Entity<EmailView>,
    /// The listener for the selected account. Replacing or dropping it cancels
    /// the previous one, including its network work on tokio.
    mail_task: Option<Task<()>>,
    /// The Gmail message currently being opened. Only one at a time: clicking
    /// another email drops (cancels) the previous request.
    open_task: Option<Task<()>>,
    opening_message_id: Option<String>,
    active_account_id: Option<String>,
    list_scroll: UniformListScrollHandle,
}

impl Inbox {
    pub fn new(
        state: Entity<AppState>,
        email_view: Entity<EmailView>,
        theme: Entity<Theme>,
        cx: &mut Context<Self>,
    ) -> Inbox {
        cx.observe(&state, |this, _state, cx| {
            this.sync_selected_account(cx);
            cx.notify();
        })
        .detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        let mut inbox = Self {
            emails: Vec::new(),
            loading: false,
            state,
            theme,
            email_view,
            mail_task: None,
            open_task: None,
            opening_message_id: None,
            active_account_id: None,
            list_scroll: UniformListScrollHandle::new(),
        };
        inbox.sync_selected_account(cx);
        inbox
    }

    fn selected_account(&self, cx: &Context<Self>) -> Option<Account> {
        let state = self.state.read(cx);
        match state.selected_sidebar_email {
            Some(SidebarEmail::Temp(index)) => {
                state.temp_email.get(index).cloned().map(Account::Temp)
            }
            Some(SidebarEmail::Google(index)) => state
                .google_accounts
                .get(index)
                .cloned()
                .map(Account::Google),
            _ => None,
        }
    }

    /// Starts a listener when the selected account changes, or clears the
    /// inbox when nothing is selected.
    fn sync_selected_account(&mut self, cx: &mut Context<Self>) {
        match self.selected_account(cx) {
            Some(account) => {
                if self.active_account_id.as_deref() != Some(account.key().as_str()) {
                    match account {
                        Account::Temp(account) => self.start_mail_listener(account, cx),
                        Account::Google(account) => self.start_google_listener(account, cx),
                    }
                }
            }
            None => {
                if self.active_account_id.is_some() || !self.emails.is_empty() {
                    self.mail_task = None;
                    self.cancel_open();
                    self.active_account_id = None;
                    self.emails.clear();
                    self.loading = false;
                    self.close_email(cx);
                }
            }
        }
    }

    /// Shared setup when switching to an account: cancel the old listener and
    /// any email being opened, show cached mail straight away.
    fn switch_to(&mut self, account_key: &str, cx: &mut Context<Self>) {
        self.mail_task = None;
        self.cancel_open();
        self.active_account_id = Some(account_key.to_string());
        self.emails = self
            .state
            .read(cx)
            .email_cache
            .get(account_key)
            .cloned()
            .unwrap_or_default();
        self.close_email(cx);
        self.loading = true;
        cx.notify();
    }

    fn start_google_listener(&mut self, mut account: GoogleAccount, cx: &mut Context<Self>) {
        let account_key = format!("google:{}", account.email);
        self.switch_to(&account_key, cx);

        let io = crate::runtime::spawn(async move {
            let result = get_gmail_mail(&mut account, 25).await;
            (account, result)
        });

        self.mail_task = Some(cx.spawn(async move |this, cx| {
            let result = io.await;

            let _ = this.update(cx, |inbox, cx| {
                if inbox.active_account_id.as_deref() != Some(account_key.as_str()) {
                    return;
                }
                inbox.loading = false;

                match result {
                    Ok((account, Ok(emails))) => {
                        inbox.store_google_account(account, cx);
                        inbox.merge_emails(emails);
                    }
                    Ok((_, Err(error))) => eprintln!("Failed to retrieve Gmail: {error:#}"),
                    Err(error) => eprintln!("Gmail task stopped: {error}"),
                }

                cx.notify();
            });
        }));
    }

    fn start_mail_listener(&mut self, account: TempEmail, cx: &mut Context<Self>) {
        let account_key = format!("temp:{}", account.id);
        let account_id = account.id.clone();
        self.switch_to(&account_key, cx);

        // The network side (token refresh, fetch, live SSE stream) runs on
        // tokio and sends results back over a channel.
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let io = crate::runtime::spawn(temp_mail_listener(account, sender));

        self.mail_task = Some(cx.spawn(async move |this, cx| {
            // Held for the life of this task; dropping it aborts the tokio side.
            let _io = io;

            while let Some(event) = receiver.recv().await {
                let updated = this.update(cx, |inbox, cx| {
                    inbox.handle_temp_event(&account_key, &account_id, event, cx)
                });
                if updated.is_err() {
                    break;
                }
            }
        }));
    }

    fn handle_temp_event(
        &mut self,
        account_key: &str,
        account_id: &str,
        event: TempMailEvent,
        cx: &mut Context<Self>,
    ) {
        if self.active_account_id.as_deref() != Some(account_key) {
            return;
        }

        match event {
            TempMailEvent::Token(token) => {
                self.state.update(cx, |state, _cx| {
                    if let Some(saved) = state.temp_email.iter_mut().find(|a| a.id == account_id) {
                        saved.token = token;
                    }
                    state.persist();
                });
            }
            TempMailEvent::Emails(emails) => {
                self.merge_emails(emails);
                self.persist_emails(cx);
                self.loading = false;
            }
            TempMailEvent::Failed => self.loading = false,
        }

        cx.notify();
    }

    fn open_email(&mut self, message_id: &str, cx: &mut Context<Self>) {
        let Some(email) = self
            .emails
            .iter()
            .find(|email| email.id == message_id)
            .cloned()
        else {
            return;
        };

        let google_account = {
            let state = self.state.read(cx);
            match state.selected_sidebar_email {
                Some(SidebarEmail::Google(index)) => state.google_accounts.get(index).cloned(),
                _ => None,
            }
        };

        // Temp mail, or a Gmail message whose body we already fetched.
        let Some(mut account) = google_account.filter(|_| email.body.is_empty()) else {
            self.cancel_open();
            self.show_email(email, cx);
            return;
        };

        if self.opening_message_id.as_deref() == Some(message_id) {
            return; // already loading this one
        }

        // Replacing the task drops (cancels) any previous open request.
        self.opening_message_id = Some(email.id.clone());
        let message_id = email.id.clone();
        let io = crate::runtime::spawn(async move {
            let result = get_gmail_message(&mut account, &message_id).await;
            (account, result)
        });

        self.open_task = Some(cx.spawn(async move |this, cx| {
            let result = io.await;

            let _ = this.update(cx, |inbox, cx| {
                inbox.opening_message_id = None;
                match result {
                    Ok((account, Ok(full_email))) => {
                        inbox.store_google_account(account, cx);
                        // Keep the body so reopening this email is instant.
                        inbox.merge_emails(vec![full_email.clone()]);
                        inbox.show_email(full_email, cx);
                    }
                    Ok((_, Err(error))) => eprintln!("Failed to load Gmail message: {error:#}"),
                    Err(error) => eprintln!("Gmail message task stopped: {error}"),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn cancel_open(&mut self) {
        self.open_task = None;
        self.opening_message_id = None;
    }

    fn show_email(&mut self, email: Email, cx: &mut Context<Self>) {
        self.email_view
            .update(cx, |view, cx| view.show(Some(email.clone()), cx));
        self.state.update(cx, |state, cx| {
            state.selected_message = Some(email);
            cx.notify();
        });
    }

    fn close_email(&mut self, cx: &mut Context<Self>) {
        self.email_view.update(cx, |view, cx| view.show(None, cx));
        self.state.update(cx, |state, _cx| {
            state.selected_message = None;
        });
    }

    /// Saves a (possibly token-refreshed) Google account back into state.
    /// Looks it up by email rather than index, so it can't panic if the
    /// account list changed while the request was running.
    fn store_google_account(&mut self, account: GoogleAccount, cx: &mut Context<Self>) {
        self.state.update(cx, |state, _cx| {
            if let Some(saved) = state
                .google_accounts
                .iter_mut()
                .find(|saved| saved.email == account.email)
            {
                *saved = account;
            }
        });
    }

    fn merge_emails(&mut self, emails: Vec<Email>) {
        for email in emails {
            if let Some(existing) = self
                .emails
                .iter_mut()
                .find(|existing| existing.id == email.id)
            {
                if existing.body.is_empty() || !email.body.is_empty() {
                    *existing = email;
                }
            } else {
                self.emails.push(email);
            }
        }

        self.emails
            .sort_by(|left, right| right.created_at.cmp(&left.created_at));
    }

    fn persist_emails(&self, cx: &mut Context<Self>) {
        if let Some(account_key) = self.active_account_id.clone() {
            let emails = self.emails.clone();
            self.state.update(cx, |state, _cx| {
                state.email_cache.insert(account_key, emails);
                state.persist();
            });
        }
    }

    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = self.theme.read(cx).clone();

        self.emails[range.start.min(self.emails.len())..range.end.min(self.emails.len())]
            .iter()
            .map(|email| {
                let message_id = email.id.clone();
                let is_opening = self.opening_message_id.as_deref() == Some(email.id.as_str());

                div()
                    .id(format!("email-{}", email.id))
                    .w_full()
                    .h(px(52.0))
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(theme.border))
                    .cursor_pointer()
                    .on_click(cx.listener(move |inbox, _event, _window, cx| {
                        inbox.open_email(&message_id, cx);
                    }))
                    // Sender
                    .child(
                        div()
                            .w(px(220.0))
                            .text_size(px(14.0))
                            .text_color(rgb(theme.text_muted))
                            .child(truncate_text(&sender_name(&email.from), 28)),
                    )
                    // Subject
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .ml_auto()
                            .text_size(px(14.0))
                            .text_color(rgb(theme.text_muted))
                            .child(truncate_text(&email.subject, 48)),
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .ml(px(6.0))
                            .text_size(px(12.0))
                            .text_color(rgb(0x777777))
                            .child(if is_opening {
                                "Opening…".to_string()
                            } else {
                                email_date(&email.created_at)
                            }),
                    )
                    .into_any_element()
            })
            .collect()
    }
}

/// Runs on tokio. Refreshes the token, loads mail, then listens to mail.tm's
/// live stream and reloads whenever something arrives. Stops when the UI side
/// goes away (channel closed) or the task is aborted.
async fn temp_mail_listener(mut account: TempEmail, sender: mpsc::UnboundedSender<TempMailEvent>) {
    if let Ok(token) = refresh_token(&account).await {
        account.token = token.clone();
        if sender.send(TempMailEvent::Token(token)).is_err() {
            return;
        }
    }

    match get_mail(&account).await {
        Ok(emails) => {
            if sender.send(TempMailEvent::Emails(emails)).is_err() {
                return;
            }
        }
        Err(error) => {
            eprintln!("Failed to retrieve mail: {error:#}");
            let _ = sender.send(TempMailEvent::Failed);
        }
    }

    let url = format!(
        "https://mercure.mail.tm/.well-known/mercure?topic=/accounts/{}",
        account.id
    );
    let request = crate::runtime::http_streaming()
        .get(&url)
        .bearer_auth(&account.token)
        .header(reqwest::header::ACCEPT, "text/event-stream");

    let mut events = match EventSource::new(request) {
        Ok(events) => events,
        Err(error) => {
            eprintln!("Failed to open temporary mail stream: {error}");
            return;
        }
    };

    while let Some(event) = events.next().await {
        if sender.is_closed() {
            break;
        }

        match event {
            Ok(Event::Open) => {}
            Ok(Event::Message(_)) => match get_mail(&account).await {
                Ok(emails) => {
                    if sender.send(TempMailEvent::Emails(emails)).is_err() {
                        break;
                    }
                }
                Err(error) => eprintln!("Failed to refresh temporary mail: {error:#}"),
            },
            Err(error) => eprintln!("Temporary mail connection error: {error}"),
        }
    }

    events.close();
}

fn sender_name(sender: &str) -> String {
    if let Some((name, _)) = sender.split_once('<') {
        let name = name.trim().trim_matches('"');
        if !name.is_empty() {
            return name.to_string();
        }
    }

    sender
        .split_once('@')
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| sender.to_string())
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let text: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{text}...")
    } else {
        text
    }
}

fn email_date(value: &str) -> String {
    let date = value
        .parse::<i64>()
        .ok()
        .and_then(|milliseconds| {
            DateTime::from_timestamp_millis(milliseconds).map(|date| date.with_timezone(&Local))
        })
        .or_else(|| {
            DateTime::parse_from_rfc3339(value)
                .ok()
                .map(|date| date.with_timezone(&Local))
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.fZ")
                .ok()
                .and_then(|date| Local.from_local_datetime(&date).single())
        });

    let Some(date) = date else {
        return String::new();
    };

    let now = Local::now();
    if date.date_naive() == now.date_naive() {
        date.format("%-I:%M %p").to_string()
    } else {
        date.format("%-d %b").to_string()
    }
}

impl Render for Inbox {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx).clone();
        let selected_account = self.state.read(cx).selected_sidebar_email;
        let has_selected_account = selected_account.is_some();

        let placeholder = |text: String| {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(rgb(0x777777))
                        .child(text),
                )
                .into_any_element()
        };

        let content = if !has_selected_account {
            placeholder("Select an inbox".to_string())
        } else if self.emails.is_empty() {
            placeholder(if self.loading {
                let label = match selected_account {
                    Some(SidebarEmail::Google(index)) => self
                        .state
                        .read(cx)
                        .google_accounts
                        .get(index)
                        .map(|account| account.email.clone())
                        .unwrap_or_else(|| "Gmail".to_string()),
                    Some(SidebarEmail::Temp(index)) => self
                        .state
                        .read(cx)
                        .temp_email
                        .get(index)
                        .map(|account| account.address.clone())
                        .unwrap_or_else(|| "temporary email".to_string()),
                    _ => "inbox".to_string(),
                };
                format!("Loading {label}")
            } else {
                "No messages".to_string()
            })
        } else {
            // Only the rows on screen are built and laid out.
            uniform_list(
                "email-list",
                self.emails.len(),
                cx.processor(|inbox, range: Range<usize>, _window, cx| {
                    inbox.render_rows(range, cx)
                }),
            )
            .track_scroll(&self.list_scroll)
            .size_full()
            .into_any_element()
        };

        div()
            .size_full()
            .min_h(px(0.0))
            .bg(rgb(theme.background))
            .flex()
            .flex_col()
            .child(div().flex_1().min_h(px(0.0)).w_full().child(content))
    }
}
