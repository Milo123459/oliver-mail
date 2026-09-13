use gpui::{
    App, Context, Entity, TitlebarOptions, Window, WindowOptions, div, prelude::*, px, rgb, size,
};

use crate::models::{Email, TempEmail, Theme};
use crate::ui::{EmailView, Inbox, MailTopBar, Sidebar, TopBar};

pub struct MailApp {
    pub sidebar: Entity<Sidebar>,
    pub topbar: Entity<TopBar>,
    pub mailtopbar: Entity<MailTopBar>,
    pub inbox: Entity<Inbox>,
    pub email_view: Entity<EmailView>,
    pub state: Entity<AppState>,
    pub theme: Entity<Theme>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub temp_email: Vec<TempEmail>,
    pub selected_email: Option<usize>,
    pub selected_message: Option<Email>,
    pub selected_sidebar_email: Option<SidebarEmail>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarEmail {
    Mail(usize),
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
            |_, cx| {
                let theme = cx.new(|_| Theme::load());
                let state = cx.new(|_| AppState {
                    temp_email: Vec::new(),
                    selected_email: None,
                    selected_message: None,
                    selected_sidebar_email: None,
                });
                let sidebar = cx.new(|_| Sidebar {
                    state: state.clone(),
                    theme: theme.clone(),
                });
                let topbar = cx.new(|_| TopBar {
                    theme: theme.clone(),
                    settings_window: None,
                });
                let mailtopbar = cx.new(|_| MailTopBar {
                    theme: theme.clone(),
                });

                let email_view = cx.new(|_| EmailView::new(state.clone(), theme.clone()));
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
                })
            },
        )
        .unwrap();
    }
}

impl Render for MailApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx).clone();
        let content = if self.state.read(cx).selected_message.is_some() {
            self.email_view.clone().into_any_element()
        } else {
            self.inbox.clone().into_any_element()
        };

        div()
            .size_full()
            .bg(rgb(Theme::color(&theme.background)))
            .text_color(rgb(Theme::color(&theme.text_muted)))
            .font_family("Lilex")
            .flex()
            .flex_col()
            .child(self.topbar.clone())
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .child(self.mailtopbar.clone())
                            .child(div().flex_1().w_full().child(content)),
                    )
                    .child(self.sidebar.clone()),
            )
    }
}
