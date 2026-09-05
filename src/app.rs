use gpui::{
    App, Entity, Context, Window, WindowOptions, div, prelude::*, px,
    rgb, size,
};

use crate::ui::{TopBar, Sidebar};

pub struct MailApp {
    sidebar: Entity<Sidebar>,
    topbar: Entity<TopBar>,
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

                cx.new(|_| MailApp {
                    sidebar,
                    topbar,
                })
            },
        )
        .unwrap();
    }
}

impl Render for MailApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .bg(rgb(0x000000))
            .text_color(rgb(0xffffff))
            .font_family("Lilex")
            .child(self.sidebar.clone())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(self.topbar.clone()),
            )
    }
}