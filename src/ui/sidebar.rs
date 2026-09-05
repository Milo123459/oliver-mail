use gpui::{
    div, prelude::*, px, rgb, Window
};

use crate::models::create_account;
pub struct Sidebar;

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .id("generate-email")
                    .p(px(10.0))
                    .bg(rgb(0x222222))
                    .cursor_pointer()
                    .on_click(cx.listener(|_this, _event, _window, cx| {
                        println!("Generate temporary email clicked!");

                        cx.spawn(async move |_this, _cx| {
                            match create_account().await {
                                Ok(email) => {
                                    println!("Created: {} {}", email.address, email.password);
                                }
                                Err(error) => {
                                    println!("Failed: {}", error);
                                }
                            }

                            Ok::<(), anyhow::Error>(())
                        })
                        .detach();  
                    }))
                    .child("generate temporary email"),
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