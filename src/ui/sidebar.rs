use gpui::{ClipboardItem, Context, Entity, Render, Window, div, prelude::*, px, rgb, svg, deferred, anchored, Anchor};

use crate::app::SidebarEmail;
use crate::models::{Theme, create_account, login};
use crate::ui::Popout;
// The right-hand panel: Gmail accounts and temp-mail addresses. Clicking an
// account selects it (the inbox reacts through its observer on `state`);
// clicking the already-selected one copies its address.
pub struct Sidebar {
    pub state: Entity<crate::app::AppState>,
    pub theme: Entity<Theme>,
    pub popout: Entity<Popout>,
    temp_menu_open: bool,
    mail_menu_open: bool,
    temp_info_open: Option<usize>,
    temp_email_hovered: Option<usize>,
}

impl Sidebar {
    pub fn new(state: Entity<crate::app::AppState>, theme: Entity<Theme>, popout: Entity<Popout>, cx: &mut Context<Self>) -> Self {
        // This view is cached, so it must re-render itself when what it
        // shows changes.
        // e.g. a new account was added or a different one selected, so redraw
        // the list.
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&popout, |_, _, cx| cx.notify()).detach();
        Self { state, theme, popout, temp_menu_open: false, mail_menu_open: false, temp_info_open: None, temp_email_hovered: None }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, root_cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(root_cx).clone();
        let temp_email_state = self.state.clone();
        let google_state = self.state.clone();
        let hover_state = self.state.clone();
        let popout = self.popout.clone();
        let selected_sidebar_email = self.state.read(root_cx).selected_sidebar_email;
        let google_accounts = self.state.read(root_cx).google_accounts.clone();

        let temporary_emails = self
            .state
            .read(root_cx)
            .temp_email
            .iter()
            .enumerate()
            .map(|(index, email)| {
                let app_state = self.state.clone();
                let email_address = email.address.clone();
                let password = email.password.clone();

                let is_selected =
                    selected_sidebar_email == Some(SidebarEmail::Temp(index));

                let is_info_open =
                    self.temp_info_open == Some(index);

                div()
                    .id(format!("temp-email-{index}"))
                    .group(format!("temp-email-group-{index}"))
                    .px(px(8.0))
                    .py(px(6.0))
                    .text_size(px(12.0))
                    .text_color(rgb(theme.text_muted))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .hover(|row| {
                        row.bg(rgb(theme.selected_option))
                            .text_color(rgb(theme.text_muted))
                    })
                    .when(is_selected, |row| {
                        row.bg(rgb(theme.selected_option))
                            .text_color(rgb(theme.text))
                    })
                    .cursor_pointer()
                    .on_hover(root_cx.listener(
                        move |this, hovered, _window, cx| {
                            this.temp_email_hovered = if *hovered {
                                Some(index)
                            } else {
                                None
                            };
                            cx.notify();
                        },
                    ))
                    .on_click(move |_event, _window, cx| {
                        app_state.update(cx, |state, cx| {
                            state.selected_email = Some(index);
                            state.selected_sidebar_email =
                                Some(SidebarEmail::Temp(index));
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(email.address.clone()),
                    )
                    .when(self.temp_email_hovered == Some(index), |row| {
                        row.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .child(
                                div()
                                    .id(format!("copy-temp-email-{index}"))
                                    .w(px(22.0))
                                    .h(px(22.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .hover(|button| {
                                        button.bg(rgb(theme.background))
                                    })
                                    .cursor_pointer()
                                    .on_click({
                                        let email_address = email_address.clone();

                                        move |_event, _window, cx| {
                                            cx.write_to_clipboard(
                                                ClipboardItem::new_string(
                                                    email_address.clone(),
                                                ),
                                            );
                                        }
                                    })
                                    .child(
                                        svg()
                                            .path("images/copy.svg")
                                            .w(px(13.0))
                                            .h(px(13.0))
                                            .text_color(rgb(theme.text_muted)),
                                    ),
                            )
                            .child(
                                div()
                                    .id(format!("settings-temp-email-{index}"))
                                    .w(px(22.0))
                                    .h(px(22.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .hover(|button| {
                                        button.bg(rgb(theme.background))
                                    })
                                    .cursor_pointer()
                                    .on_click(root_cx.listener(
                                        move |this, _event, _window, cx| {
                                            this.temp_info_open =
                                                if this.temp_info_open == Some(index) {
                                                    None
                                                } else {
                                                    Some(index)
                                                };

                                            cx.notify();
                                        },
                                    ))
                                    .child(
                                        svg()
                                            .path("images/settings.svg")
                                            .w(px(13.0))
                                            .h(px(13.0))
                                            .text_color(rgb(theme.text_muted)),
                                    ),
                            )
                        )
                    })
                    .when(is_info_open, |row| {
                        row.child(
                            deferred(
                                anchored()
                                    .anchor(Anchor::TopRight)
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(28.0))
                                            .right(px(0.0))
                                            .w(px(280.0))
                                            .p(px(12.0))
                                            .bg(rgb(theme.background))
                                            .border(px(1.0))
                                            .border_color(rgb(theme.border))
                                            .rounded(px(6.0))
                                            .shadow_lg()
                                            .occlude()
                                            .flex()
                                            .flex_col()
                                            .gap(px(10.0))
                                            .child(
                                                div()
                                                    .text_size(px(13.0))
                                                    .text_color(rgb(theme.text))
                                                    .child("Temporary Email"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(4.0))
                                                    .child(
                                                        div()
                                                            .text_size(px(10.0))
                                                            .text_color(
                                                                rgb(theme.text_muted),
                                                            )
                                                            .child("Email"),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.0))
                                                            .px(px(8.0))
                                                            .py(px(6.0))
                                                            .bg(rgb(theme.surface))
                                                            .rounded(px(4.0))
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .child(
                                                                        email_address
                                                                            .clone(),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id(format!(
                                                                        "info-copy-email-{index}"
                                                                    ))
                                                                    .w(px(22.0))
                                                                    .h(px(22.0))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .cursor_pointer()
                                                                    .on_click({
                                                                        let email_address =
                                                                            email_address
                                                                                .clone();

                                                                        move |_event,
                                                                            _window,
                                                                            cx| {
                                                                            cx.write_to_clipboard(
                                                                                ClipboardItem::new_string(
                                                                                    email_address
                                                                                        .clone(),
                                                                                ),
                                                                            );
                                                                        }
                                                                    })
                                                                    .child(
                                                                        svg()
                                                                            .path(
                                                                                "images/copy.svg",
                                                                            )
                                                                            .w(px(13.0))
                                                                            .h(px(13.0))
                                                                            .text_color(
                                                                                rgb(
                                                                                    theme.text_muted,
                                                                                ),
                                                                            ),
                                                                    ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(4.0))
                                                    .child(
                                                        div()
                                                            .text_size(px(10.0))
                                                            .text_color(
                                                                rgb(theme.text_muted),
                                                            )
                                                            .child("Password"),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(6.0))
                                                            .px(px(8.0))
                                                            .py(px(6.0))
                                                            .bg(rgb(theme.surface))
                                                            .rounded(px(4.0))
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .child(
                                                                        password.clone(),
                                                                    ),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id(format!(
                                                                        "info-copy-password-{index}"
                                                                    ))
                                                                    .w(px(22.0))
                                                                    .h(px(22.0))
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .cursor_pointer()
                                                                    .on_click({
                                                                        let password =
                                                                            password
                                                                                .clone();

                                                                        move |_event,
                                                                            _window,
                                                                            cx| {
                                                                            cx.write_to_clipboard(
                                                                                ClipboardItem::new_string(
                                                                                    password
                                                                                        .clone(),
                                                                                ),
                                                                            );
                                                                        }
                                                                    })
                                                                    .child(
                                                                        svg()
                                                                            .path(
                                                                                "images/copy.svg",
                                                                            )
                                                                            .w(px(13.0))
                                                                            .h(px(13.0))
                                                                            .text_color(
                                                                                rgb(
                                                                                    theme.text_muted,
                                                                                ),
                                                                            ),
                                                                    ),
                                                            ),
                                                    ),
                                            ),
                                    ),
                            )
                            .priority(10),
                        )
                    })
            })
            .collect::<Vec<_>>();

        div()
            .w(px(360.0))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(theme.surface))
            .on_mouse_move(move |_event, _window, cx| {
                hover_state.update(cx, |_state, cx| cx.notify());
            })
            .border_l(px(1.0))
            .border_color(rgb(theme.border))
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
                            .text_color(rgb(theme.text))
                            .child("Mail")
                            .child(
                                div()
                                    .h(px(25.0))
                                    .px(px(5.0))
                                    .rounded(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(|this| this.bg(rgb(theme.selected_option)))
                                    .id("add-email")
                                    .cursor_pointer()

                                    .on_click(root_cx.listener(
                                        move |this, _event, _window, cx| {
                                            this.mail_menu_open = !this.mail_menu_open;
                                            cx.notify();
                                        },
                                    ))
                                    .child(
                                        svg()
                                            .path("images/add.svg")
                                            .text_color(rgb(theme.text_muted))
                                            .w(px(10.0))
                                            .h(px(10.0)),
                                    )
                                    .when(self.mail_menu_open, |button| {
                                        button.child(
                                            deferred(
                                                anchored()
                                                    .anchor(Anchor::TopLeft)
                                                    .child(
                                                        div()
                                                            .absolute()
                                                            .top(px(10.0))
                                                            .right(px(0.0))
                                                            .w(px(190.0))
                                                            .py(px(5.0))
                                                            .bg(rgb(theme.background))
                                                            .border(px(1.0))
                                                            .border_color(rgb(theme.border))
                                                            .rounded(px(6.0))
                                                            .shadow_lg()
                                                            .occlude()
                                                            .child(
                                                                div()
                                                                    .id("temp-menu-option-1")
                                                                    .px(px(10.0))
                                                                    .py(px(7.0))
                                                                    .text_size(px(12.0))
                                                                    .text_color(rgb(theme.text))
                                                                    .hover(|item| {
                                                                        item.bg(rgb(theme.selected_option))
                                                                    })
                                                                    .cursor_pointer()
                                                                    .on_click(root_cx.listener(
                                                                        move |_this, _event, _window, cx| {
                                                                            _this.mail_menu_open = false;

                                                                            let google_state = google_state.clone();
                                                                            if google_state.read(cx).google_login_status.as_deref()
                                                                                == Some("Opening Google login...")
                                                                            {
                                                                                return;
                                                                            }
                                                                            google_state.update(cx, |state, cx| {
                                                                                state.google_login_status =
                                                                                    Some("Opening Google login...".to_string());
                                                                                cx.notify();
                                                                            });
                                                                            // login() runs an axum server and HTTP calls, so it runs on tokio.
                                                                            // Start the work on tokio first, then wait for it from a gpui task.
                                                                            let io = crate::runtime::spawn(login());
                                                                            cx.spawn(async move |_this, cx2| {
                                                                                let result = io
                                                                                    .await
                                                                                    .map_err(anyhow::Error::from)
                                                                                    .and_then(|result| result);
                                                                                match result {
                                                                                    Ok(account) => {
                                                                                        google_state.update(cx2, |state, cx| {
                                                                                            state.google_accounts.push(account);
                                                                                            state.persist();
                                                                                            state.google_login_status = None;
                                                                                            state.selected_sidebar_email =
                                                                                                Some(SidebarEmail::Google(
                                                                                                    state.google_accounts.len() - 1,
                                                                                                ));
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                    Err(error) => {
                                                                                        google_state.update(cx2, |state, cx| {
                                                                                            state.google_login_status =
                                                                                                Some(format!(
                                                                                                    "Google login failed: {error:#}"
                                                                                                ));
                                                                                            cx.notify();
                                                                                        });
                                                                                        eprintln!("Google login failed: {error:#}");
                                                                                    }
                                                                                }
                                                                                Ok::<(), anyhow::Error>(())
                                                                            })
                                                                            .detach();
                                                                        },
                                                                    ))
                                                                    .child("Gmail"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id("temp-menu-option-2")
                                                                    .px(px(10.0))
                                                                    .py(px(7.0))
                                                                    .text_size(px(12.0))
                                                                    .text_color(rgb(theme.text))
                                                                    .hover(|item| {
                                                                        item.bg(rgb(theme.selected_option))
                                                                    })
                                                                    .cursor_pointer()
                                                                    .on_click(root_cx.listener(
                                                                        move |this, _event, _window, _cx| {
                                                                            println!("Menu option 2 clicked");
                                                                            this.mail_menu_open = false;
                                                                        },
                                                                    ))
                                                                    .child("Yahoo"),
                                                            ),
                                                    ),
                                            )
                                            .priority(1),
                                        )
                                    })
                            ),
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .pl(px(14.0))
                            .border_l(px(1.0))
                            .border_color(rgb(theme.border))
                            .children(google_accounts.iter().enumerate().map(
                                |(index, account)| {
                                    let app_state = self.state.clone();
                                    let email_address = account.email.clone();
                                    let is_selected =
                                        selected_sidebar_email == Some(SidebarEmail::Google(index));
                                    div()
                                        .id(format!("google-account-{index}"))
                                        .px(px(8.0))
                                        .py(px(6.0))
                                        .text_size(px(12.0))
                                        .text_color(rgb(theme.text_muted))
                                        .hover(|row| {
                                            row.bg(rgb(theme.selected_option))
                                                .text_color(rgb(theme.text_muted))
                                        })
                                        .when(is_selected, |row| {
                                            row.bg(rgb(theme.selected_option))
                                                .text_color(rgb(theme.text))
                                        })
                                        .cursor_pointer()
                                        .on_click(root_cx.listener(
                                            move |_this, _event, _window, cx| {
                                                if is_selected && !email_address.is_empty() {
                                                    cx.write_to_clipboard(
                                                        ClipboardItem::new_string(
                                                            email_address.clone(),
                                                        ),
                                                    );
                                                    return;
                                                }
                                                app_state.update(cx, |state, cx| {
                                                    state.selected_email = None;
                                                    state.selected_sidebar_email =
                                                        Some(SidebarEmail::Google(index));
                                                    cx.notify();
                                                });
                                            },
                                        ))
                                        .child(if account.email.is_empty() {
                                            "Gmail".to_string()
                                        } else {
                                            account.email.clone()
                                        })
                                },
                            )),
                    )
                    .when_some(
                        self.state.read(root_cx).google_login_status.clone(),
                        |this, status| {
                            this.child(
                                div()
                                    .px(px(8.0))
                                    .text_size(px(11.0))
                                    .text_color(rgb(theme.text_muted))
                                    .child(status),
                            )
                        },
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
                            .text_color(rgb(theme.text))
                            .child("Temp Emails")
                            .child(
                                div()
                                    .h(px(25.0))
                                    .px(px(5.0))
                                    .rounded(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(|this| this.bg(rgb(theme.selected_option)))
                                    .id("generate-email")
                                    .cursor_pointer()
                                    .on_click(root_cx.listener(
                                        move |this, _event, _window, cx| {
                                            this.temp_menu_open = !this.temp_menu_open;
                                            cx.notify();
                                        },
                                    ))
                                    .child(
                                        svg()
                                            .path("images/add.svg")
                                            .text_color(rgb(theme.text))
                                            .w(px(10.0))
                                            .h(px(10.0)),
                                    )
                                    .when(self.temp_menu_open, |button| {
                                        button.child(
                                            deferred(
                                                anchored()
                                                    .anchor(Anchor::TopLeft)
                                                    .child(
                                                        div()
                                                            .absolute()
                                                            .top(px(10.0))
                                                            .right(px(0.0))
                                                            .w(px(190.0))
                                                            .py(px(5.0))
                                                            .bg(rgb(theme.background))
                                                            .border(px(1.0))
                                                            .border_color(rgb(theme.border))
                                                            .rounded(px(6.0))
                                                            .shadow_lg()
                                                            .occlude()
                                                            .child(
                                                                div()
                                                                    .id("temp-menu-option-1")
                                                                    .px(px(10.0))
                                                                    .py(px(7.0))
                                                                    .text_size(px(12.0))
                                                                    .text_color(rgb(theme.text))
                                                                    .hover(|item| {
                                                                        item.bg(rgb(theme.selected_option))
                                                                    })
                                                                    .cursor_pointer()
                                                                    .on_click(root_cx.listener(
                                                                        move |_this: &mut Sidebar, _event, _window, cx| {
                                                                            _this.temp_menu_open = false;
                                                                            let app_state = temp_email_state.clone();

                                                                            // Same pattern as login above: the network call runs on tokio, and the
                                                                            // gpui task below waits for it and then updates `state` on the UI side.
                                                                            //
                                                                            // `io.await` gives `Result<Result<TempEmail>, JoinError>`: the outer
                                                                            // error means the tokio task panicked/was cancelled, the inner one that
                                                                            // mail.tm said no. `map_err` + `and_then` flatten both into one
                                                                            // `Result` so we only need one `match`.
                                                                            let io = crate::runtime::spawn(create_account());
                                                                            cx.spawn(async move |_this, cx2| {
                                                                                let result = io
                                                                                    .await
                                                                                    .map_err(anyhow::Error::from)
                                                                                    .and_then(|result| result);
                                                                                match result {
                                                                                    Ok(email) => {
                                                                                        app_state.update(cx2, |state, cx| {
                                                                                            state.temp_email.push(email);
                                                                                            state.persist();
                                                                                            cx.notify();
                                                                                        });
                                                                                    }
                                                                                    Err(error) => {
                                                                                        eprintln!(
                                                                                            "Failed to create temp email: {error:#}"
                                                                                        );
                                                                                    }
                                                                                }
                                                                                Ok::<(), anyhow::Error>(())
                                                                            })
                                                                            .detach();
                                                                        },
                                                                    ))
                                                                    .child("Quick Generate"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .id("temp-menu-option-2")
                                                                    .px(px(10.0))
                                                                    .py(px(7.0))
                                                                    .text_size(px(12.0))
                                                                    .text_color(rgb(theme.text))
                                                                    .hover(|item| {
                                                                        item.bg(rgb(theme.selected_option))
                                                                    })
                                                                    .cursor_pointer()
                                                                    .on_click(root_cx.listener(
                                                                        move |this, _event, _window, cx| {
                                                                            println!("Menu option 2 clicked");
                                                                            this.temp_menu_open = false;
                                                                            popout.update(cx, |popout, cx| {
                                                                                popout.show(cx);
                                                                                cx.notify();
                                                                            });
                                                                        },
                                                                    ))
                                                                    .child("Custom Generate"),
                                                            ),
                                                    ),
                                            )
                                            .priority(1),
                                        )
                                    })
                            ),
                    )
                    .child(
                        div()
                            .ml(px(8.0))
                            .pl(px(14.0))
                            .border_l(px(1.0))
                            .border_color(rgb(theme.border))
                            .children(temporary_emails),
                    ),
            )
            .into_any_element()
    }
}
