use gpui::{
    div,
    prelude::*,
    px,
    rgb,
    Context,
    Render,
    Window,
};

use crate::models::{get_mail, Email, TempEmail};

pub struct Inbox {
    pub account: TempEmail,
    pub emails: Vec<Email>,
    pub loading: bool,
}

impl Inbox {
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let account = self.account.clone();

        self.loading = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            match get_mail(&account).await {
                Ok(emails) => {
                    this.update(cx, |inbox, cx| {
                        inbox.emails = emails;
                        inbox.loading = false;
                        cx.notify();
                    })?;
                }

                Err(error) => {
                    eprintln!("Failed to retrieve mail: {}", error);

                    this.update(cx, |inbox, cx| {
                        inbox.loading = false;
                        cx.notify();
                    })?;
                }
            }

            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }
}

impl Render for Inbox {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .bg(rgb(0x111111))
            .flex()
            .flex_col()

            // Header
            .child(
                div()
                    .w_full()
                    .h(px(64.0))
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(0x252525))
                    .child(
                        div()
                            .text_size(px(20.0))
                            .text_color(rgb(0xffffff))
                            .child("Inbox")
                    )
            )

            // Email list
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .children(
                        self.emails
                            .iter()
                            .map(|email| {
                                div()
                                    .w_full()
                                    .h(px(64.0))
                                    .px(px(24.0))
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(rgb(0x222222))

                                    // Sender
                                    .child(
                                        div()
                                            .w(px(400.0))
                                            .text_size(px(14.0))
                                            .text_color(rgb(0xdce0e5))
                                            .child(email.from.clone())
                                    )

                                    // Subject
                                    .child(
                                        div()
                                            .ml_auto()
                                            .text_size(px(14.0))
                                            .text_color(rgb(0xdce0e5))
                                            .child(email.subject.clone())
                                    )

                                    // Date
                                    /*.child(
                                        div()
                                            .w(px(100.0))
                                            .text_color(rgb(0x777777))
                                            .child(email.created_at.clone())
                                    )*/
                                    .into_any_element()
                            })
                    )
            )
    }
}