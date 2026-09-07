use gpui::{Context, Window, div, prelude::*, px, rgb};

pub struct TopBar;

impl Render for TopBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(35.0))
            .flex()
            .items_center()
            .bg(rgb(0x181818))
            //.border_b(px(1.0))
            //.border_color(rgb(0x2a2a2a))
            // Active Selection
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xffffff))
                    .text_size(px(15.0))
                    .bg(rgb(0x111111))
                    .mb(px(-1.0))
                    .pb(px(1.0))
                    .child("inbox")
                    .border_r(px(1.0))
                    .border_color(rgb(0x464B57)),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .text_size(px(15.0))
                    .child("starred")
                    .border_b(px(1.0))
                    .border_r(px(1.0))
                    .border_color(rgb(0x464B57)), /*.hover(|this| {
                                                      println!("Testing");
                                                      this.bg(rgb(0x202020))
                                                          .text_color(rgb(0xd0d0d0))
                                                  }),*/
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .text_size(px(15.0))
                    .child("drafts")
                    .border_b(px(1.0))
                    .border_r(px(1.0))
                    .border_color(rgb(0x464B57)),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .text_size(px(15.0))
                    .child("sent")
                    .border_b(px(1.0))
                    .border_r(px(1.0))
                    .border_color(rgb(0x464B57)),
            )
            // Account 2
            .child(
                div()
                    .h_full()
                    .px(px(18.0))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xaaaaaa))
                    .text_size(px(15.0))
                    .child("trash")
                    .border_b(px(1.0))
                    .border_r(px(1.0))
                    .border_color(rgb(0x464B57)),
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
                    .child("+")
                    .border_b(px(1.0))
                    .border_color(rgb(0x464B57)),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .border_b(px(1.0))
                    .border_color(rgb(0x464B57)),
            )
            .into_any_element()
    }
}
