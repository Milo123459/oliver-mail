use gpui::{Entity, Window, div, prelude::*, px, rgb, svg};

use crate::app::SidebarEmail;
use crate::models::{Theme, create_account};
pub struct Sidebar {
    pub state: Entity<crate::app::AppState>,
    pub theme: Entity<Theme>,
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, root_cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(root_cx).clone();
        let app_state = self.state.clone();
        let mail_account_0_state = self.state.clone();
        let mail_account_1_state = self.state.clone();
        let selected_sidebar_email = self.state.read(root_cx).selected_sidebar_email;
        let temporary_emails = self
            .state
            .read(root_cx)
            .temp_email
            .iter()
            .enumerate()
            .map(|(index, email)| {
                let app_state = self.state.clone();
                div()
                    .id(format!("temp-email-{index}"))
                    .px(px(8.0))
                    .py(px(6.0))
                    .text_size(px(12.0))
                    .text_color(rgb(Theme::color(&theme.text_muted)))
                    .when(
                        selected_sidebar_email == Some(SidebarEmail::Temp(index)),
                        |row| {
                            row.bg(rgb(Theme::color(&theme.selected_option)))
                                .text_color(rgb(Theme::color(&theme.text)))
                        },
                    )
                    .hover(|row| {
                        row.bg(rgb(Theme::color(&theme.selected_option)))
                            .text_color(rgb(Theme::color(&theme.text_muted)))
                    })
                    .cursor_pointer()
                    .on_click(move |_event, _window, cx| {
                        app_state.update(cx, |state, cx| {
                            state.selected_email = Some(index);
                            state.selected_sidebar_email = Some(SidebarEmail::Temp(index));
                            cx.notify();
                        });
                    })
                    .child(email.address.clone())
            })
            .collect::<Vec<_>>();

        div()
            .w(px(360.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(Theme::color(&theme.surface)))
            .border_l(px(1.0))
            .border_color(rgb(Theme::color(&theme.border)))
            .child(
                div()
                    .w_full()
                    .px(px(14.0))
                    .py(px(12.0))
                    .child(
                        div()
                            .h(px(30.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .text_size(px(14.0))
                            .text_color(rgb(Theme::color(&theme.text)))
                            .child("Mail")
                            .child(
                                div()
                                    .h(px(25.0))
                                    .px(px(5.0))
                                    .rounded(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(|this| {
                                        this.bg(rgb(Theme::color(&theme.selected_option)))
                                    })
                                    .child(
                                        svg()
                                            .data(include_bytes!("../../assets/images/add.svg"))
                                            .text_color(rgb(Theme::color(&theme.text_muted)))
                                            .w(px(10.0))
                                            .h(px(10.0)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .pl(px(14.0))
                            .border_l(px(1.0))
                            .border_color(rgb(Theme::color(&theme.border)))
                            .child(
                                div()
                                    .id("mail-account-0")
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .text_size(px(12.0))
                                    .text_color(rgb(Theme::color(&theme.text_muted)))
                                    .when(
                                        selected_sidebar_email == Some(SidebarEmail::Mail(0)),
                                        |row| {
                                            row.bg(rgb(Theme::color(&theme.selected_option)))
                                                .text_color(rgb(Theme::color(&theme.text)))
                                        },
                                    )
                                    .hover(|row| {
                                        row.bg(rgb(Theme::color(&theme.selected_option)))
                                            .text_color(rgb(Theme::color(&theme.text_muted)))
                                    })
                                    .cursor_pointer()
                                    .on_click(root_cx.listener(
                                        move |_this, _event, _window, cx| {
                                            let app_state = mail_account_0_state.clone();
                                            app_state.update(cx, |state, cx| {
                                                state.selected_sidebar_email =
                                                    Some(SidebarEmail::Mail(0));
                                                cx.notify();
                                            });
                                        },
                                    ))
                                    .child("oliver@gmail.com"),
                            )
                            .child(
                                div()
                                    .id("mail-account-1")
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .text_size(px(12.0))
                                    .text_color(rgb(Theme::color(&theme.text_muted)))
                                    .when(
                                        selected_sidebar_email == Some(SidebarEmail::Mail(1)),
                                        |row| {
                                            row.bg(rgb(Theme::color(&theme.selected_option)))
                                                .text_color(rgb(Theme::color(&theme.text)))
                                        },
                                    )
                                    .hover(|row| {
                                        row.bg(rgb(Theme::color(&theme.selected_option)))
                                            .text_color(rgb(Theme::color(&theme.text)))
                                    })
                                    .cursor_pointer()
                                    .on_click(root_cx.listener(
                                        move |_this, _event, _window, cx| {
                                            let app_state = mail_account_1_state.clone();
                                            app_state.update(cx, |state, cx| {
                                                state.selected_sidebar_email =
                                                    Some(SidebarEmail::Mail(1));
                                                cx.notify();
                                            });
                                        },
                                    ))
                                    .child("rem@googlemail.com"),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .px(px(14.0))
                    .py(px(12.0))
                    .child(
                        div()
                            .h(px(30.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .text_size(px(14.0))
                            .text_color(rgb(Theme::color(&theme.text)))
                            .child("Temp Emails")
                            .child(
                                div()
                                    .h(px(25.0))
                                    .px(px(5.0))
                                    .rounded(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(|this| {
                                        this.bg(rgb(Theme::color(&theme.selected_option)))
                                    })
                                    .id("generate-email")
                                    .cursor_pointer()
                                    .on_click(root_cx.listener(
                                        move |_this, _event, _window, cx| {
                                            let app_state = app_state.clone();

                                            cx.spawn(async move |_this, cx2| {
                                                match create_account().await {
                                                    Ok(email) => {
                                                        app_state.update(cx2, |state, cx| {
                                                            state.temp_email.push(email);
                                                            cx.notify();
                                                        });
                                                    }
                                                    Err(error) => {
                                                        println!("Failed: {}", error);
                                                    }
                                                }
                                                Ok::<(), anyhow::Error>(())
                                            })
                                            .detach();
                                        },
                                    ))
                                    .child(
                                        svg()
                                            .data(include_bytes!("../../assets/images/add.svg"))
                                            .text_color(rgb(Theme::color(&theme.text)))
                                            .w(px(10.0))
                                            .h(px(10.0)),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .pl(px(14.0))
                            .border_l(px(1.0))
                            .border_color(rgb(Theme::color(&theme.border)))
                            .children(temporary_emails),
                    ),
            )
            .into_any_element()
    }
}
