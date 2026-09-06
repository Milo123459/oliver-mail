use gpui::{
    App, Entity, Context, Window, WindowOptions, div, prelude::*, px,
    rgb, size,
};

use crate::models::{TempEmail};
use crate::ui::{TopBar, Sidebar, Inbox};

pub struct MailApp {
    sidebar: Entity<Sidebar>,
    topbar: Entity<TopBar>,
    inbox: Entity<Inbox>,
}

impl MailApp {
    pub fn open(cx: &mut App) {
        let font = include_bytes!("assets/fonts/Lilex[wght].ttf");

        cx.text_system()
            .add_fonts(vec![std::borrow::Cow::Borrowed(font.as_slice())])
            .expect("Failed to load Lilex font");

        cx.open_window(
            WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(
                    gpui::Bounds::centered(
                        None, 
                        size(px(1200.0), 
                        px(800.0)), 
                        cx),
                )),
                ..Default::default()
            },
            |_, cx| {
                let sidebar = cx.new(|_| Sidebar);
                let topbar = cx.new(|_| TopBar);

                let account = TempEmail {
                    address: "metropolitanemelyne@web-library.net".to_string(),
                    password: "evC*I<<>#w".to_string(),
                };

                let inbox = cx.new(|_| Inbox {
                    account,
                    emails: Vec::new(),
                    loading: false,
                });

                inbox.update(cx, |inbox, cx| {
                    inbox.refresh(cx);
                });

                cx.new(|_| MailApp {
                    sidebar,
                    topbar,
                    inbox,
                })
            },
        )
        .unwrap();
    }
}

impl Render for MailApp {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x000000))
            .text_color(rgb(0xffffff))
            .font_family("Lilex")
            .flex()
            .flex_col()

            .child(self.topbar.clone())
            
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()

                    .child(self.inbox.clone())
                    .child(self.sidebar.clone())
            )
    }
}