use gpui::{
    Anchor, App, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton, Render, Window,
    anchored, deferred, div, prelude::*, px, rgb,
};
use crate::app::AppState;
use crate::models::{Theme, create_account_with_credentials, get_domains};

// Focus ring for the two text inputs.
//
// Deliberately not a theme colour: none of the shipped themes have an accent
// slot. Their `selected` is a selection *background* (`#181818` on dark,
// `#E4E7EB` on light and zed), so reusing it would make the focused field flash
// near-black or near-white. A mid blue reads as "this one has the keyboard"
// against both the dark and the light themes, and 0x3b82f6 keeps roughly a 3:1
// contrast ratio against #1e1e1e, so the ring stays visible for low vision.
const FOCUS_RING: u32 = 0x3b82f6;

pub struct Popout {
    pub theme: gpui::Entity<Theme>,
    pub state: gpui::Entity<AppState>,
    email_focus: FocusHandle,
    password_focus: FocusHandle,
    open: bool,
    email_value: String,
    password_value: String,
    selected_domain: String,
    domains: Vec<String>,
    domain_menu_open: bool,
    error: Option<String>,
    generating: bool,
}

impl Popout {
    pub fn new(theme: gpui::Entity<Theme>, state: gpui::Entity<AppState>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        Self {
            theme,
            state,
            email_focus: cx.focus_handle(),
            password_focus: cx.focus_handle(),
            open: false,
            email_value: String::new(),
            password_value: String::new(),
            selected_domain: String::new(),
            domains: Vec::new(),
            domain_menu_open: false,
            error: None,
            generating: false,
        }
    }

    /// True while the "Custom Generate" panel is showing. `MailApp` reads this
    /// to decide whether to draw the dimmed backdrop.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Close the panel and any dropdown inside it. Called when the user clicks
    /// the backdrop.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if self.open || self.domain_menu_open {
            self.open = false;
            self.domain_menu_open = false;
            self.error = None;
            cx.notify();
        }
    }

    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        self.domain_menu_open = false;
        self.error = None;
        self.email_value.clear();
        self.password_value.clear();

        if self.domains.is_empty() {
            let task = crate::runtime::spawn(get_domains());

            cx.spawn(async move |this, cx| {
                match task.await {
                    Ok(Ok(domains)) => {
                        this.update(cx, |popout, cx| {
                            if popout.selected_domain.is_empty()
                                && let Some(first_domain) = domains.first()
                            {
                                popout.selected_domain = first_domain.clone();
                            }

                            popout.domains = domains;
                            cx.notify();
                        })?;
                    }
                    Ok(Err(error)) => {
                        eprintln!("Failed to get Mail.tm domains: {error:#}");

                        this.update(cx, |popout, cx| {
                            popout.error =
                                Some(format!("Failed to load domains: {error:#}"));
                            cx.notify();
                        })?;
                    }
                    Err(error) => {
                        eprintln!("Failed to get Mail.tm domains task: {error}");

                        this.update(cx, |popout, cx| {
                            popout.error =
                                Some(format!("Failed to load domains: {error}"));
                            cx.notify();
                        })?;
                    }
                }

                Ok::<(), anyhow::Error>(())
            })
            .detach();
        }

        cx.notify();
    }

    fn handle_email_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "backspace" => {
                self.email_value.pop();
                self.error = None;
            }
            "escape" => {
                if self.domain_menu_open {
                    self.domain_menu_open = false;
                } else {
                    self.open = false;
                }
            }
            _ => {
                if let Some(character) = &event.keystroke.key_char
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                {
                    self.email_value.push_str(character);
                    self.error = None;
                }
            }
        }

        window.focus(&self.email_focus, cx);
        cx.notify();
    }

    fn handle_password_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "backspace" => {
                self.password_value.pop();
                self.error = None;
            }
            "escape" => {
                self.open = false;
            }
            _ => {
                if let Some(character) = &event.keystroke.key_char
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.platform
                {
                    self.password_value.push_str(character);
                    self.error = None;
                }
            }
        }

        window.focus(&self.password_focus, cx);
        cx.notify();
    }

    fn generate_account(&mut self, cx: &mut Context<Self>) {
        self.error = None;

        if self.email_value.trim().is_empty() {
            self.error = Some("Email username is required".to_string());
            cx.notify();
            return;
        }

        if self.selected_domain.is_empty() {
            self.error = Some("Please select a domain".to_string());
            cx.notify();
            return;
        }

        if self.password_value.is_empty() {
            self.error = Some("Password is required".to_string());
            cx.notify();
            return;
        }

        if self.password_value.len() < 8 {
            self.error = Some("Password must be at least 8 characters".to_string());
            cx.notify();
            return;
        }

        if self.generating {
            return;
        }

        let username = self.email_value.trim().to_lowercase();
        let domain = self.selected_domain.clone();
        let password = self.password_value.clone();

        let address = format!("{}@{}", username, domain);

        println!("Creating temporary email: {}", address);

        self.generating = true;

        let state = self.state.clone();

        let io = crate::runtime::spawn(
            create_account_with_credentials(address, password),
        );

        cx.spawn(async move |this, cx| {
            match io.await {
                Ok(Ok(email)) => {
                    state.update(cx, |state, cx| {
                        state.temp_email.push(email);
                        state.persist();
                        cx.notify();
                    });

                    this.update(cx, |popout, cx| {
                        popout.generating = false;
                        popout.error = None;
                        popout.open = false;
                        popout.domain_menu_open = false;
                        cx.notify();
                    })?;
                }

                Ok(Err(error)) => {
                    this.update(cx, |popout, cx| {
                        popout.generating = false;
                        popout.error = Some(error.to_string());
                        cx.notify();
                    })?;
                }

                Err(error) => {
                    this.update(cx, |popout, cx| {
                        popout.generating = false;
                        popout.error = Some(error.to_string());
                        cx.notify();
                    })?;
                }
            }

            Ok::<(), anyhow::Error>(())
        })
        .detach();

        cx.notify();
    }
}

impl Focusable for Popout {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.email_focus.clone()
    }
}

impl Render for Popout {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx).clone();

        // Which text input currently has the keyboard. Read here rather than
        // with a `when_focused` style because the email field's focusable
        // element is a *child* of the box that draws the border, so the ring has
        // to be driven by the container.
        let email_focused = self.email_focus.is_focused(window);
        let password_focused = self.password_focus.is_focused(window);

        let email_value = if self.email_value.is_empty() {
            "username".to_string()
        } else {
            self.email_value.clone()
        };

        let email_color = if self.email_value.is_empty() {
            theme.text_muted
        } else {
            theme.text
        };

        let password_display = if self.password_value.is_empty() {
            "Password".to_string()
        } else {
            "•".repeat(self.password_value.chars().count())
        };

        let password_color = if self.password_value.is_empty() {
            theme.text_muted
        } else {
            theme.text
        };

        let selected_domain = self.selected_domain.clone();

        div()
            .id("app-popout-layer")
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .left(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .when(self.open, |layer| {
                layer.child(
                    div()
                        .id("app-popout-panel")
                        .w(px(340.0))
                        .p(px(16.0))
                        .bg(rgb(theme.surface))
                        .border_1()
                        .border_color(rgb(theme.border))
                        .shadow_lg()
                        .occlude()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(rgb(theme.text))
                                .child("Custom Generate"),
                        )
                        .child(
                            div()
                                .mt(px(16.0))
                                .text_size(px(12.0))
                                .text_color(rgb(theme.text_muted))
                                .child("Email address"),
                        )
                        .child(
                            div()
                                .id("email-input-container")
                                .relative()
                                .mt(px(6.0))
                                .w_full()
                                .h(px(38.0))
                                .flex()
                                .items_center()
                                .bg(rgb(theme.background))
                                // Thicker as well as coloured, so the ring is
                                // obvious even if the colours are close. The box
                                // has a fixed height and centres its contents, so
                                // the extra pixel does not shift the text.
                                .when(
                                    email_focused,
                                    |this| {
                                        this.border_2()
                                            .border_color(rgb(FOCUS_RING))
                                    },
                                )
                                .when(!email_focused, |this| {
                                    this.border_1()
                                        .border_color(rgb(theme.border))
                                })
                                .rounded(px(5.0))
                                .child(
                                    div()
                                        .id("email-input")
                                        .flex_1()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .px(px(10.0))
                                        .text_size(px(13.0))
                                        .text_color(rgb(email_color))
                                        .track_focus(&self.email_focus)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                |popout, _, window, cx| {
                                                    popout.domain_menu_open = false;

                                                    window.focus(
                                                        &popout.email_focus,
                                                        cx,
                                                    );
                                                },
                                            ),
                                        )
                                        .on_key_down(cx.listener(
                                            |popout, event, window, cx| {
                                                popout.handle_email_key(
                                                    event,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ))
                                        .child(email_value),
                                )
                                .child(
                                    div()
                                        .h(px(22.0))
                                        .w(px(1.0))
                                        .bg(rgb(theme.border)),
                                )
                                .child(
                                    div()
                                        .relative()
                                        .h_full()
                                        .px(px(9.0))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(
                                                |popout, _, _, cx| {
                                                    popout.domain_menu_open =
                                                        !popout.domain_menu_open;

                                                    cx.notify();
                                                },
                                            ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .text_color(rgb(theme.text))
                                                .child(selected_domain),
                                        )
                                        .child(
                                            div()
                                                .ml(px(6.0))
                                                .text_size(px(9.0))
                                                .text_color(rgb(theme.text_muted))
                                                .child("▼"),
                                        )
                                        .when(
                                            self.domain_menu_open,
                                            |selector| {
                                                selector.child(
                                                    deferred(
                                                        anchored()
                                                            .anchor(
                                                                Anchor::BottomLeft,
                                                            )
                                                            .child(
                                                                div()
                                                                    .w(px(170.0))
                                                                    .max_h(px(220.0))
                                                                    .py(px(4.0))
                                                                    .bg(rgb(
                                                                        theme.surface,
                                                                    ))
                                                                    .border_1()
                                                                    .border_color(
                                                                        rgb(
                                                                            theme.border,
                                                                        ),
                                                                    )
                                                                    .rounded(
                                                                        px(5.0),
                                                                    )
                                                                    .shadow_lg()
                                                                    .occlude()
                                                                    // Clicking anywhere outside this dropdown -- but still inside the
                                                                    // modal -- closes just the dropdown. The backdrop in app.rs
                                                                    // handles clicks outside the modal itself.
                                                                    .on_mouse_down_out(cx.listener(|popout, _, _, cx| {
                                                                        popout.domain_menu_open = false;
                                                                        cx.notify();
                                                                    }))
                                                                    .children(
                                                                        self.domains
                                                                            .iter()
                                                                            .enumerate()
                                                                            .map(
                                                                                |(
                                                                                    index,
                                                                                    domain,
                                                                                )| {
                                                                                    let domain =
                                                                                        domain.clone();
                                                                                    let domain_for_click =
                                                                                        domain.clone();
                                                                                    let domain_id =
                                                                                        format!(
                                                                                            "domain-{}",
                                                                                            index
                                                                                        );

                                                                                    div()
                                                                                        .id(
                                                                                            domain_id,
                                                                                        )
                                                                                        .w_full()
                                                                                        .px(
                                                                                            px(9.0),
                                                                                        )
                                                                                        .py(
                                                                                            px(7.0),
                                                                                        )
                                                                                        .text_size(
                                                                                            px(12.0),
                                                                                        )
                                                                                        .text_color(
                                                                                            rgb(
                                                                                                theme.text,
                                                                                            ),
                                                                                        )
                                                                                        .hover(
                                                                                            |item| {
                                                                                                item.bg(
                                                                                                    rgb(
                                                                                                        theme.selected_option,
                                                                                                    ),
                                                                                                )
                                                                                            },
                                                                                        )
                                                                                        .on_mouse_down(
                                                                                            MouseButton::Left,
                                                                                            cx.listener(
                                                                                                move |popout, _, _, cx| {
                                                                                                    popout.selected_domain =
                                                                                                        domain_for_click.clone();
                                                                                                    popout.domain_menu_open =
                                                                                                        false;
                                                                                                    popout.error =
                                                                                                        None;
                                                                                                    cx.notify();
                                                                                                },
                                                                                            ),
                                                                                        )
                                                                                        .child(
                                                                                            domain,
                                                                                        )
                                                                                },
                                                                            ),
                                                                    ),
                                                            ),
                                                    )
                                                    .priority(1),
                                                )
                                            },
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(14.0))
                                .text_size(px(12.0))
                                .text_color(rgb(theme.text_muted))
                                .child("Password"),
                        )
                        .child(
                            div()
                                .id("password-input")
                                .mt(px(6.0))
                                .w_full()
                                .h(px(38.0))
                                .flex()
                                .items_center()
                                .px(px(10.0))
                                .bg(rgb(theme.background))
                                .when(password_focused, |this| {
                                    this.border_2()
                                        .border_color(rgb(FOCUS_RING))
                                })
                                .when(!password_focused, |this| {
                                    this.border_1()
                                        .border_color(rgb(theme.border))
                                })
                                .rounded(px(5.0))
                                .text_size(px(13.0))
                                .text_color(rgb(password_color))
                                .track_focus(&self.password_focus)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |popout, _, window, cx| {
                                            popout.domain_menu_open = false;

                                            window.focus(
                                                &popout.password_focus,
                                                cx,
                                            );
                                        },
                                    ),
                                )
                                .on_key_down(cx.listener(
                                    |popout, event, window, cx| {
                                        popout.handle_password_key(
                                            event,
                                            window,
                                            cx,
                                        );
                                    },
                                ))
                                .child(password_display),
                        )
                        .when_some(self.error.clone(), |element, error| {
                            element.child(
                                div()
                                    .mt(px(10.0))
                                    .w_full()
                                    .p(px(9.0))
                                    .bg(rgb(theme.background))
                                    .border_1()
                                    .border_color(rgb(theme.border))
                                    .rounded(px(5.0))
                                    .text_size(px(12.0))
                                    .text_color(rgb(theme.text_muted))
                                    .child(error),
                            )
                        })
                        .child(
                            div()
                                .id("submit-button")
                                .mt(px(16.0))
                                .w_full()
                                .h(px(38.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(rgb(theme.surface))
                                .rounded(px(5.0))
                                .text_size(px(13.0))
                                .text_color(rgb(theme.text))
                                .cursor_pointer()
                                .hover(|item| {
                                    item.bg(rgb(theme.surface_hover))
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|popout, _, _, cx| {
                                        popout.generate_account(cx);
                                    }),
                                )
                                .child(if self.generating {
                                    "Creating..."
                                } else {
                                    "Generate"
                                }),
                        ),
                )
            })
    }
}
