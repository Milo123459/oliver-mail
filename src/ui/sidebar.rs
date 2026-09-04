use gpui::{
    div, prelude::*, px, rgb, Window
};

pub struct Sidebar;

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(220.0))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .border_r(px(1.0))
            .border_color(rgb(0x222222))
            .child(
                div()
                    .p(px(20.0))
                    .text_size(px(24.0))
                    .mb(px(20.0))
                    .child("Mail"),
            )
            .child(
                div()
                    .p(px(10.0))
                    //.rounded(px(8.0))
                    .bg(rgb(0x222222))
                    .child("oliver@gmail.com"),
            )
            .child(
                div()
                    .p(px(10.0))
                    .child("example@work"),
            )
            .into_any_element()
    }
}