use gpui::{
    App, ClipboardItem, Context, Entity, IntoElement, MouseButton, StyleRefinement, TitlebarOptions,
    Window, WindowOptions, div, prelude::*, px, rgb, rgba, size, svg,
};
use std::collections::HashMap;

use crate::models::{Email, GoogleAccount, TempEmail, Theme};
use crate::ui::{EmailView, Inbox, MailTopBar, Popout, Sidebar, TopBar};

// The root view of the window. It owns a handle to every child view plus the
// shared state and theme.
//
// `Entity<T>` is gpui's handle to a piece of state that lives inside the app
// (a bit like an `Rc<RefCell<T>>` managed by gpui). Cloning an `Entity` just
// clones the handle, not the data. You read it with `entity.read(cx)` and
// change it with `entity.update(cx, |value, cx| ...)`.
pub struct MailApp {
    pub sidebar: Entity<Sidebar>,
    pub topbar: Entity<TopBar>,
    pub mailtopbar: Entity<MailTopBar>,
    pub inbox: Entity<Inbox>,
    pub email_view: Entity<EmailView>,
    pub state: Entity<AppState>,
    pub theme: Entity<Theme>,
    pub popout: Entity<Popout>,
}

// One labelled row of the temp-mail details modal: a caption, the value in a box
// of its own, and a copy button. The value gets `min_w(0)` so a long address
// clips at the box edge instead of widening the modal.
fn temp_info_field(label: &str, value: &str, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(theme.text_muted))
                .child(label.to_string()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .py(px(6.0))
                .bg(rgb(theme.background))
                .rounded(px(4.0))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .child(value.to_string()),
                )
                .child(
                    div()
                        .id(format!("temp-info-copy-{}", label.to_lowercase()))
                        .w(px(22.0))
                        .h(px(22.0))
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .hover(|button| button.bg(rgb(theme.surface_hover)))
                        .cursor_pointer()
                        .on_click({
                            let value = value.to_string();
                            move |_event, _window, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    value.clone(),
                                ));
                            }
                        })
                        .child(
                            svg()
                                .path("images/copy.svg")
                                .w(px(13.0))
                                .h(px(13.0))
                                .text_color(rgb(theme.text_muted)),
                        ),
                ),
        )
}

// Data shared by every view: accounts, cached mail, what's selected.
// It lives in one `Entity<AppState>` that each view holds a handle to.
// When something changes it, the code calls `cx.notify()` inside the
// `update`, and every view that observes `state` gets told (see the
// `cx.observe(&state, ...)` calls in each view's `new()`).
#[derive(Clone, Debug)]
pub struct AppState {
    pub temp_email: Vec<TempEmail>,
    pub google_accounts: Vec<GoogleAccount>,
    // Key is "temp:<account id>" or "google:<email address>".
    pub email_cache: HashMap<String, Vec<Email>>,
    pub temp_starred: HashMap<String, Vec<String>>,
    pub selected_email: Option<usize>,
    // `Some` while an email is open. MailApp's render uses this to decide
    // whether to show the inbox list or the email view.
    pub selected_message: Option<Email>,
    pub selected_sidebar_email: Option<SidebarEmail>,
    pub google_login_status: Option<String>,
    pub mail_filter: MailFilter,
    // `Some(index)` while a temp-mail details modal is showing. This lives here
    // rather than on `Sidebar` because the modal is drawn centred on the whole
    // window, and the sidebar is a 360px-wide cached view -- anything rendered
    // inside it can only ever be centred on the sidebar, never the window.
    pub temp_info_open: Option<usize>,
}

impl AppState {
    fn from_storage(data: crate::storage::StoredData) -> Self {
        let selected_sidebar_email = if !data.google_accounts.is_empty() {
            Some(SidebarEmail::Google(0))
        } else if !data.temp_email.is_empty() {
            Some(SidebarEmail::Temp(0))
        } else {
            None
        };

        Self {
            temp_email: data.temp_email,
            google_accounts: data.google_accounts,
            email_cache: data.emails,
            temp_starred: data.temp_starred,
            selected_email: match selected_sidebar_email {
                Some(SidebarEmail::Temp(index)) => Some(index),
                _ => None,
            },
            selected_message: None,
            selected_sidebar_email,
            google_login_status: None,
            mail_filter: MailFilter::Inbox,
            temp_info_open: None,
        }
    }

    // Writes everything to disk synchronously (on the UI thread). Fine for now,
    // but with a lot of cached mail it could cause small hitches.
    pub fn persist(&self) {
        crate::storage::save(&crate::storage::StoredData {
            temp_email: self.temp_email.clone(),
            google_accounts: self.google_accounts.clone(),
            emails: self.email_cache.clone(),
            temp_starred: self.temp_starred.clone(),
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailFilter {
    Inbox,
    Starred,
    Drafts,
    Sent,
    Trash,
}

impl MailFilter {
    pub fn gmail_label(self) -> &'static str {
        match self {
            Self::Inbox => "INBOX",
            Self::Starred => "STARRED",
            Self::Drafts => "DRAFT",
            Self::Sent => "SENT",
            Self::Trash => "TRASH",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarEmail {
    //Mail(usize),
    Google(usize),
    Temp(usize),
}

impl MailApp {
    pub fn open(cx: &mut App) {
        let font = include_bytes!("../assets/fonts/Lilex[wght].ttf");

        cx.text_system()
            .add_fonts(vec![std::borrow::Cow::Borrowed(font.as_slice())])
            .expect("Failed to load Lilex font");

        cx.open_window(
            WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::centered(
                    None,
                    size(px(1200.0), px(800.0)),
                    cx,
                ))),

                titlebar: Some(TitlebarOptions {
                    title: Some("Mail".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),

                ..Default::default()
            },
            // Every view is created with `cx.new(...)`. Views that need to react to
            // changes get `cx` in their constructor so they can register observers.
            |_, cx| {
                let theme = cx.new(|_| Theme::load());
                let state = cx.new(|_| AppState::from_storage(crate::storage::load()));
                let popout = cx.new(|cx| Popout::new(theme.clone(), state.clone(), cx));
                let sidebar = cx.new(|cx| Sidebar::new(state.clone(), theme.clone(), popout.clone(), cx));
                let topbar = cx.new(|cx| TopBar::new(theme.clone(), state.clone(), cx));
                let mailtopbar = cx.new(|cx| MailTopBar::new(state.clone(), theme.clone(), cx));

                // The inbox gets a handle to the email view so that clicking an email can
                // tell the email view what to show.
                let email_view = cx.new(|cx| EmailView::new(theme.clone(), cx));
                let inbox =
                    cx.new(|cx| Inbox::new(state.clone(), email_view.clone(), theme.clone(), cx));

                cx.new(|_| MailApp {
                    sidebar,
                    topbar,
                    mailtopbar,
                    inbox,
                    email_view,
                    state,
                    theme,
                    popout,
                })
            },
        )
        .unwrap();
    }
}

impl Render for MailApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx).clone();
        let hover_state = self.state.clone();

        // ---- Why `.cached(...)`? ------------------------------------------
        //
        // By default gpui re-renders a view AND every view above it whenever
        // anything inside it changes, and that includes hover effects. Since
        // MailApp is the parent of everything, hovering a single button used
        // to rebuild the whole window: sidebar, both top bars, every email row
        // and the full email body. That's where the CPU/GPU spikes came from.
        //
        // `.cached(style)` tells gpui: "reuse what this child drew last frame
        // unless the child itself was notified". Now hovering a sidebar button
        // only re-renders the sidebar; the inbox and email view are reused.
        //
        // Two rules that come with caching:
        //
        // 1. The `style` passed to `.cached()` must give the view a definite
        //    size (e.g. `size_full()`, or a fixed width/height), because gpui
        //    lays out a cached view from that style instead of measuring its
        //    contents.
        //
        // 2. A cached view does NOT redraw just because data it reads changed.
        //    If the sidebar reads `state` and `state` changes, the sidebar
        //    keeps showing the old frame until someone calls `cx.notify()` on
        //    the sidebar. That's why every view registers observers in its
        //    `new()`:
        //
        //        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        //
        //    = "whenever `state` is notified, notify me too".
        //    `.detach()` keeps the subscription alive for the life of the view
        //    (otherwise it would be dropped, and unsubscribed, straight away).
        //
        // MailApp itself is the window's root and is always re-rendered when
        // the window redraws, so it doesn't need observers of its own.

        // A click anywhere outside a popout should dismiss it. One full-window
        // scrim does that for every popout at once, and it does two jobs:
        //
        // 1. It dims the window, so the popout you're looking at is clearly the
        //    thing in focus.
        // 2. It swallows the outside click. Without this, clicking "outside"
        //    would close the menu *and* activate whatever is underneath -- open
        //    an email, select an account. `.occlude()` is what stops the click
        //    travelling on: it makes the hitbox `BlockMouse`, so gpui's hit test
        //    breaks here instead of continuing into the elements below.
        //
        // Paint order is what makes this safe. This is a plain child of the root,
        // so it paints after the top bars, the sidebar and the inbox. The anchored
        // menus are `deferred(...)`, and gpui prepaints *and* paints those in a
        // separate pass after the whole main tree, so they stay on top of the
        // scrim and stay clickable. The generate panel below is a later sibling,
        // so it also lands on top. Since gpui hit-tests in reverse paint order,
        // a click on a menu reaches the menu and never reaches the scrim.
        //
        // The temp-mail details panel used to be an `anchored` popover built inside
        // the sidebar, which is why it sat off to one side rather than centred: the
        // sidebar is a 360px-wide cached view, so anything drawn inside it can only
        // ever be centred on the sidebar. It is now built here, on the root, and
        // centred on the window exactly like the generate panel.
        //
        // Only the two *centred modals* dim the window. The little menus that drop
        // down from the sidebar's "+" buttons have no scrim, because a dimmed
        // window reads as "something modal is open" and those are not -- they close
        // themselves with `on_mouse_down_out` instead.
        //
        // Read each flag in its own `let` so the read guard is dropped before the
        // next one is taken.
        let temp_info_open = self.state.read(cx).temp_info_open;
        let generate_popout_open = self.popout.read(cx).is_open();
        let any_modal_open = temp_info_open.is_some() || generate_popout_open;

        // Built only while open, so nothing is measured when it is not on screen.
        // `and_then` yields `None` if the index no longer points at a temp email,
        // which happens if the list shrinks while the modal is open.
        let temp_info_modal = temp_info_open.and_then(|index| {
            let email = self.state.read(cx).temp_email.get(index)?.clone();

            Some(
                div()
                    .id("temp-info-modal")
                    .w(px(340.0))
                    .p(px(16.0))
                    .bg(rgb(theme.surface))
                    .border_1()
                    .border_color(rgb(theme.border))
                    .rounded(px(6.0))
                    .shadow_lg()
                    .occlude()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(rgb(theme.text))
                            .child("Temporary Email"),
                    )
                    .child(temp_info_field("Email", &email.address, &theme))
                    .child(temp_info_field("Password", &email.password, &theme)),
            )
        });

        let content = if self.state.read(cx).selected_message.is_some() {
            self.email_view
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element()
        } else {
            self.inbox
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element()
        };

        div()
            .size_full()
            .bg(rgb(theme.background))
            .text_color(rgb(theme.text_muted))
            .font_family("Lilex")
            .on_mouse_exit(move |_event, _window, cx| {
                hover_state.update(cx, |_state, cx| cx.notify());
            })
            .flex()
            .flex_col()
            .child(
                self.topbar.clone().cached(
                    StyleRefinement::default()
                        .w_full()
                        .h(px(35.0))
                        .flex_shrink_0(),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(0.0))
                    .flex()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            // Was missing: without `flex()` the column isn't
                            // a flex container, so the content area never got
                            // a height and nothing inside could scroll.
                            .flex()
                            .flex_col()
                            .child(
                                self.mailtopbar.clone().cached(
                                    StyleRefinement::default()
                                        .w_full()
                                        .h(px(35.0))
                                        .flex_shrink_0(),
                                ),
                            )
                            // `min_h(0)` on flex children lets them shrink below their content's
                            // height; without it a long email would push the layout off-screen instead
                            // of scrolling.
                            .child(div().flex_1().w_full().min_h(px(0.0)).child(content)),
                    )
                    .child(
                        self.sidebar.clone().cached(
                            StyleRefinement::default()
                                .w(px(360.0))
                                .h_full()
                                .flex_shrink_0(),
                        ),
                    ),
            )
                    .when(any_modal_open, |root| {
                        root.child(
                            div()
                                .id("popout-backdrop")
                                .absolute()
                                .top(px(0.0))
                                .right(px(0.0))
                                .bottom(px(0.0))
                                .left(px(0.0))
                                .bg(rgba(0x00000059))
                                .occlude()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|app, _event, _window, cx| {
                                        app.state.update(cx, |state, cx| {
                                            state.temp_info_open = None;
                                            cx.notify();
                                        });
                                        app.popout.update(cx, |popout, cx| {
                                            popout.close(cx);
                                        });
                                    }),
                                ),
                        )
                    })
                    // The temp-mail details modal, centred on the window.
                    .when(temp_info_open.is_some(), |root| {
                        root.child(
                            div()
                                .id("temp-info-layer")
                                .absolute()
                                .top(px(0.0))
                                .right(px(0.0))
                                .bottom(px(0.0))
                                .left(px(0.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(temp_info_modal.unwrap()),
                        )
                    })
                    .child(self.popout.clone())
    }
}
