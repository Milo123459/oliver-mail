use gpui::{
    Context, Window, div, prelude::*, px, rgb
};

pub struct TopBar;

impl Render for TopBar {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .w_full()
            .h(px(45.0))
            .flex()
            .items_center()
            .bg(rgb(0x181818))
            .border_b(px(1.0))
            .border_color(rgb(0x2a2a2a))
            // Account 1
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .bg(rgb(0x222222))
                    .text_color(rgb(0xffffff))
                    .child("Inbox"),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .child("Starred"),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .child("Drafts"),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .child("Sent"),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .child("Trash"),
            )
            // Add account button
            .child(
                div()
                    .h_full()
                    .px(px(15.0))
                    .flex()
                    .items_center()
                    .text_size(px(20.0))
                    .text_color(rgb(0xaaaaaa))
                    .child("+"),
            )
            .into_any_element()
    }
}