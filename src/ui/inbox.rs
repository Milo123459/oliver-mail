use crate::app::AppState;
use crate::models::{Email, GoogleAccount, Theme, get_gmail_mail, get_gmail_message, get_mail};
use crate::ui::EmailView;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use futures_util::StreamExt;
use gpui::{Context, Entity, Render, Task, Window, div, prelude::*, px, rgb};
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::oneshot;

enum Account {
    Temp(crate::models::TempEmail),
    Google(usize, GoogleAccount),
}

pub struct Inbox {
    pub emails: Vec<Email>,
    pub loading: bool,
    pub state: Entity<AppState>,
    pub theme: Entity<Theme>,
    pub email_view: Entity<EmailView>,
    mail_task: Option<Task<Result<(), anyhow::Error>>>,
    cancel_sender: Option<oneshot::Sender<()>>,
    active_account_id: Option<String>,
}

impl Inbox {
    pub fn new(
        state: Entity<AppState>,
        email_view: Entity<EmailView>,
        theme: Entity<Theme>,
        cx: &mut Context<Self>,
    ) -> Inbox {
        let inbox = Self {
            emails: Vec::new(),
            loading: false,
            state: state.clone(),
            theme,
            email_view,
            mail_task: None,
            cancel_sender: None,
            active_account_id: None,
        };

        cx.observe(&state, |this, state, cx| {
            let selected_account = {
                let state = state.read(cx);

                match state.selected_sidebar_email {
                    Some(crate::app::SidebarEmail::Temp(index)) => state
                        .temp_email
                        .get(index)
                        .cloned()
                        .map(|account| (format!("temp:{}", account.id), Account::Temp(account))),
                    Some(crate::app::SidebarEmail::Google(index)) => state
                        .google_accounts
                        .get(index)
                        .cloned()
                        .map(|account| (format!("google:{index}"), Account::Google(index, account))),
                    _ => None,
                }
            };

            if let Some((account_id, account)) = selected_account {
                if this.active_account_id.as_deref() != Some(account_id.as_str()) {
                    match account {
                        Account::Temp(account) => this.start_mail_listener(account, cx),
                        Account::Google(index, account) => {
                            this.start_google_listener(index, account, cx)
                        }
                    }
                }
            } else {
                if let Some(cancel_sender) = this.cancel_sender.take() {
                    let _ = cancel_sender.send(());
                }
                this.mail_task = None;
                this.active_account_id = None;
                this.emails.clear();
                this.email_view.update(cx, |email_view, _cx| {
                    email_view.email = None;
                });
                state.update(cx, |state, _cx| {
                    state.selected_message = None;
                });
                this.loading = false;
                cx.notify();
            }
        })
        .detach();

        inbox
    }

    fn start_google_listener(
        &mut self,
        account_index: usize,
        mut account: GoogleAccount,
        cx: &mut Context<Self>,
    ) {
        if let Some(cancel_sender) = self.cancel_sender.take() {
            let _ = cancel_sender.send(());
        }
        self.mail_task = None;
        let account_key = format!("google:{account_index}");
        self.active_account_id = Some(account_key.clone());
        self.emails.clear();
        self.email_view.update(cx, |email_view, _cx| {
            email_view.email = None;
        });
        self.state.update(cx, |state, _cx| {
            state.selected_message = None;
        });
        self.loading = true;
        cx.notify();

        let state = self.state.clone();
        let task = cx.spawn(async move |this, cx| {
            match get_gmail_mail(&mut account, 25).await {
                Ok(emails) => {
                    state.update(cx, |state, cx| {
                        state.google_accounts[account_index] = account.clone();
                        cx.notify();
                    });
                    this.update(cx, |inbox, cx| {
                        if inbox.active_account_id.as_deref() != Some(account_key.as_str()) {
                            return;
                        }
                        inbox.merge_emails(emails);
                        inbox.loading = false;
                        cx.notify();
                    })?;
                }
                Err(error) => {
                    eprintln!("Failed to retrieve Gmail: {error:#}");
                    this.update(cx, |inbox, cx| {
                        if inbox.active_account_id.as_deref() != Some(account_key.as_str()) {
                            return;
                        }
                        inbox.loading = false;
                        cx.notify();
                    })?;
                }
            }

            Ok::<(), anyhow::Error>(())
        });

        self.mail_task = Some(task);
    }

    fn start_mail_listener(&mut self, account: crate::models::TempEmail, cx: &mut Context<Self>) {
        if let Some(cancel_sender) = self.cancel_sender.take() {
            let _ = cancel_sender.send(());
        }
        self.mail_task = None;
        let account_key = format!("temp:{}", account.id);
        self.active_account_id = Some(account_key.clone());
        self.emails.clear();
        self.email_view.update(cx, |email_view, _cx| {
            email_view.email = None;
        });
        self.state.update(cx, |state, _cx| {
            state.selected_message = None;
        });
        self.loading = true;
        cx.notify();

        let (cancel_sender, mut cancel_receiver) = oneshot::channel();
        self.cancel_sender = Some(cancel_sender);
        let task = cx.spawn(async move |this, cx| {
            match get_mail(&account).await {
                Ok(emails) => {
                    this.update(cx, |inbox, cx| {
                        if inbox.active_account_id.as_deref() != Some(account_key.as_str()) {
                            return;
                        }
                        inbox.merge_emails(emails);
                        inbox.loading = false;
                        cx.notify();
                    })?;
                }

                Err(error) => {
                    eprintln!("Failed to retrieve mail: {}", error);

                    this.update(cx, |inbox, cx| {
                        if inbox.active_account_id.as_deref() != Some(account_key.as_str()) {
                            return;
                        }
                        inbox.loading = false;
                        cx.notify();
                    })?;
                }
            }

            let url = format!(
                "https://mercure.mail.tm/.well-known/mercure?topic=/accounts/{}",
                account.id
            );
            let client = reqwest::Client::new();
            let request = client
                .get(&url)
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", account.token),
                )
                .header(reqwest::header::ACCEPT, "text/event-stream");
            let mut events = EventSource::new(request)?;

            loop {
                let event = tokio::select! {
                    _ = &mut cancel_receiver => break,
                    event = events.next() => event,
                };

                let Some(event) = event else {
                    break;
                };

                match event {
                    Ok(Event::Open) => {}
                    Ok(Event::Message(_)) => match get_mail(&account).await {
                        Ok(emails) => {
                            this.update(cx, |inbox, cx| {
                                if inbox.active_account_id.as_deref()
                                    != Some(account_key.as_str())
                                {
                                    return;
                                }
                                inbox.merge_emails(emails);
                                cx.notify();
                            })?;
                        }
                        Err(error) => eprintln!("Failed to refresh temporary mail: {error}"),
                    },
                    Err(error) => eprintln!("Temporary mail connection error: {error}"),
                }
            }

            Ok::<(), anyhow::Error>(())
        });

        self.mail_task = Some(task);
    }

    fn merge_emails(&mut self, emails: Vec<Email>) {
        for email in emails {
            if let Some(existing) = self
                .emails
                .iter_mut()
                .find(|existing| existing.id == email.id)
            {
                *existing = email;
            } else {
                self.emails.push(email);
            }
        }

        self.emails
            .sort_by(|left, right| right.created_at.cmp(&left.created_at));
    }
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
        .and_then(|milliseconds| DateTime::from_timestamp_millis(milliseconds).map(|date| date.with_timezone(&Local)))
        .or_else(|| DateTime::parse_from_rfc3339(value).ok().map(|date| date.with_timezone(&Local)))
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
        let loading_label = match selected_account {
            Some(crate::app::SidebarEmail::Google(index)) => self
                .state
                .read(cx)
                .google_accounts
                .get(index)
                .map(|account| account.email.clone())
                .unwrap_or_else(|| "Gmail".to_string()),
            Some(crate::app::SidebarEmail::Temp(index)) => self
                .state
                .read(cx)
                .temp_email
                .get(index)
                .map(|account| account.address.clone())
                .unwrap_or_else(|| "temporary email".to_string()),
            _ => "inbox".to_string(),
        };
        div()
            .w_full()
            .h_full()
            .min_h(px(0.0))
            .bg(rgb(Theme::color(&theme.background)))
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .when(!has_selected_account, |this| {
                        this.items_center().justify_center().child(
                            div()
                                .text_size(px(14.0))
                                .text_color(rgb(0x777777))
                                .child("Select an inbox"),
                        )
                    })
                    .when(
                        has_selected_account && self.emails.is_empty(),
                        |this| {
                            this.items_center().justify_center().child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(rgb(0x777777))
                                    .child(if self.loading {
                                        format!("Loading {loading_label}")
                                    } else {
                                        "No messages".to_string()
                                    }),
                            )
                        },
                    )
                    .when(
                        has_selected_account && !self.emails.is_empty(),
                        |this| {
                            this.children(self.emails.iter().map(|email| {
                                div()
                                    .w_full()
                                    .h(px(52.0))
                                    .px(px(24.0))
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(rgb(Theme::color(&theme.border)))
                                    .id(format!("email-{}", email.id))
                                    .cursor_pointer()
                                    .on_click({
                                        let state = self.state.clone();
                                        let email_view = self.email_view.clone();
                                        let email = email.clone();

                                        move |_event, _window, cx| {
                                            let google_index = match state.read(cx).selected_sidebar_email {
                                                Some(crate::app::SidebarEmail::Google(index)) => Some(index),
                                                _ => None,
                                            };

                                            if let Some(google_index) = google_index {
                                                let account = state.read(cx).google_accounts[google_index].clone();
                                                let state_for_task = state.clone();
                                                let email_view_for_task = email_view.clone();
                                                let message_id = email.id.clone();
                                                cx.spawn(async move |cx2| {
                                                    let mut account = account;
                                                    match get_gmail_message(&mut account, &message_id).await {
                                                        Ok(full_email) => {
                                                            state_for_task.update(cx2, |state, cx| {
                                                                state.google_accounts[google_index] = account;
                                                                state.selected_message = Some(full_email.clone());
                                                                cx.notify();
                                                            });
                                                            email_view_for_task.update(cx2, |email_view, _cx| {
                                                                email_view.email = Some(full_email);
                                                            });
                                                        }
                                                        Err(error) => eprintln!("Failed to load Gmail message: {error:#}"),
                                                    }
                                                    Ok::<(), anyhow::Error>(())
                                                }).detach();
                                            } else {
                                                email_view.update(cx, |email_view, _cx| {
                                                    email_view.email = Some(email.clone());
                                                });
                                                state.update(cx, |state, cx| {
                                                    state.selected_message = Some(email.clone());
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                    // Sender
                                    .child(
                                        div()
                                            .w(px(220.0))
                                            .text_size(px(14.0))
                                            .text_color(rgb(Theme::color(&theme.text_muted)))
                                            .child(truncate_text(&sender_name(&email.from), 28)),
                                    )
                                    // Subject
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .ml_auto()
                                            .text_size(px(14.0))
                                            .text_color(rgb(Theme::color(&theme.text_muted)))
                                            .child(truncate_text(&email.subject, 48)),
                                    )
                                    .child(
                                        div()
                                            .w(px(80.0))
                                            .ml(px(6.0))
                                            .text_size(px(12.0))
                                            .text_color(rgb(0x777777))
                                            .child(email_date(&email.created_at)),
                                    )
                                    .into_any_element()
                            }))
                        },
                    ),
            )
    }
}
