use gpui::{
    App, Context, Window, WindowOptions, div, prelude::*, px,
    rgb, size,
};

use crate::ui::{TopBar, Sidebar};

pub struct MailApp;

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
            |_, cx| cx.new(|_| Self),
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
            .child(
                div()
                    .flex_1()
                    .flex()
                    .child(_cx.new(|_| Sidebar))
                    .child(_cx.new(|_| TopBar)),
            )
    }
}