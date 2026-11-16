//! Native Floem view implementation for the Antares AI agent panel.

use std::{rc::Rc, sync::Arc};

use antares_agent::{ChatRole, PanelLifecycle, ToolStatus};
use floem::{
    views::{
        container, dyn_stack, empty, h_stack, label, scroll, stack, text,
        Decorators,
    },
    IntoView, View,
};

use crate::{
    app::clickable_icon,
    config::{color::LapceColor, icon::LapceIcons, LapceConfig},
    panel::position::PanelPosition,
    window_tab::WindowTabData,
};

/// Construct the root Floem view for the Antares AI agent panel.
pub fn antares_panel(
    window_tab_data: Rc<WindowTabData>,
    _position: PanelPosition,
) -> impl View {
    let data = window_tab_data.antares_data.clone();
    let config = window_tab_data.common.config;
    let state = data.state;

    // Header bar: Title, Status Pill, Clear button
    let header = h_stack((
        text("Antares AI").style(move |s| {
            s.font_bold()
                .padding_left(10.0)
                .color(config.get().color(LapceColor::PANEL_FOREGROUND))
        }),
        // Status Badge Pill
        container(
            label(move || state.with(|s| s.status_text().to_string())).style(
                move |s| {
                    let is_err = state.with(|s| {
                        matches!(
                            s.lifecycle,
                            PanelLifecycle::Error { .. }
                                | PanelLifecycle::AuthRequired { .. }
                        )
                    });
                    let is_streaming = state.with(|s| s.is_streaming());
                    let color = if is_err {
                        config.get().color(LapceColor::PROBLEM)
                    } else if is_streaming {
                        config.get().color(LapceColor::EDITOR_LINK)
                    } else {
                        config.get().color(LapceColor::PANEL_FOREGROUND_DIM)
                    };
                    s.font_size(11.0).color(color)
                },
            ),
        )
        .style(move |s| {
            s.padding_horiz(6.0)
                .padding_vert(2.0)
                .border_radius(4.0)
                .background(
                    config
                        .get()
                        .color(LapceColor::PANEL_CURRENT_BACKGROUND),
                )
        }),
        empty().style(|s| s.flex_grow(1.0f32)),
        // Clear Transcript Icon
        clickable_icon(
            || LapceIcons::CLOSE,
            {
                let data = data.clone();
                move || {
                    data.clear_transcript();
                }
            },
            || false,
            || false,
            || "Clear Transcript",
            config,
        ),
    ))
    .style(move |s| {
        s.width_full()
            .height(30.0)
            .items_center()
            .padding_right(6.0)
            .background(config.get().color(LapceColor::EDITOR_BACKGROUND))
            .border_bottom(1.0)
            .border_color(config.get().color(LapceColor::LAPCE_BORDER))
    });

    // Empty state suggestion view
    let empty_state = container(
        stack((
            label(|| "Antares AI Agent".to_string()).style(move |s| {
                s.font_bold()
                    .font_size(14.0)
                    .color(config.get().color(LapceColor::PANEL_FOREGROUND))
            }),
            label(|| {
                "Ask a question, propose edits, or audit security.".to_string()
            })
            .style(move |s| {
                s.font_size(12.0)
                    .margin_top(6.0)
                    .color(config.get().color(LapceColor::PANEL_FOREGROUND_DIM))
            }),
        ))
        .style(|s| s.flex_col().items_center().padding(24.0)),
    )
    .style(move |s| {
        let is_empty = state.with(|s| s.messages.is_empty());
        s.width_full()
            .items_center()
            .justify_center()
            .flex_grow(1.0f32)
            .apply_if(!is_empty, |s| s.hide())
    });

    // Transcript message list
    let transcript = scroll(
        dyn_stack(
            move || state.with(|s| s.messages.clone()),
            |msg| msg.id.clone(),
            move |msg| {
                let is_user = msg.role == ChatRole::User;
                let role_name = match msg.role {
                    ChatRole::User => "You",
                    ChatRole::Assistant => "Antares Agent",
                    ChatRole::System => "System",
                };
                let content = if msg.content.is_empty() && msg.is_streaming {
                    "Thinking...".to_string()
                } else {
                    msg.content.clone()
                };

                // Message bubble container
                stack((
                    // Header with author role
                    h_stack((label(move || role_name).style(move |s| {
                        let role_color = if is_user {
                            config.get().color(LapceColor::EDITOR_LINK)
                        } else {
                            config.get().color(LapceColor::PANEL_FOREGROUND)
                        };
                        s.font_bold().font_size(12.0).color(role_color)
                    }),)),
                    // Content text
                    label(move || content.clone()).style(move |s| {
                        s.width_full()
                            .padding_vert(4.0)
                            .selectable(true)
                            .color(config.get().color(LapceColor::PANEL_FOREGROUND))
                    }),
                    // Tool activities (if any)
                    dyn_stack(
                        move || msg.tool_activities.clone(),
                        |t| t.id.clone(),
                        move |tool| {
                            let status_str = match tool.status {
                                ToolStatus::Running => "running",
                                ToolStatus::Completed => "done",
                                ToolStatus::Failed => "failed",
                            };
                            let tool_name = tool.name.clone();
                            h_stack((label(move || {
                                format!("⚙ {}: {}", tool_name, status_str)
                            })
                            .style(move |s| {
                                s.font_size(11.0).color(
                                    config
                                        .get()
                                        .color(LapceColor::PANEL_FOREGROUND_DIM),
                                )
                            }),))
                            .style(move |s| {
                                s.padding_horiz(8.0)
                                    .padding_vert(3.0)
                                    .margin_vert(2.0)
                                    .border_radius(4.0)
                                    .background(
                                        config
                                            .get()
                                            .color(LapceColor::PANEL_HOVERED_BACKGROUND),
                                    )
                            })
                        },
                    ),
                ))
                .style(move |s| {
                    let bg = if is_user {
                        config
                            .get()
                            .color(LapceColor::PANEL_CURRENT_BACKGROUND)
                    } else {
                        config.get().color(LapceColor::PANEL_BACKGROUND)
                    };
                    s.width_full()
                        .flex_col()
                        .padding(8.0)
                        .margin_vert(4.0)
                        .border_radius(6.0)
                        .background(bg)
                })
            },
        )
        .style(|s| s.flex_col().width_full().padding(8.0)),
    )
    .style(move |s| {
        let is_empty = state.with(|s| s.messages.is_empty());
        s.flex_grow(1.0f32)
            .width_full()
            .apply_if(is_empty, |s| s.hide())
    });

    // Error or Auth Alert Banner
    let banner = container(
        label(move || {
            state.with(|s| {
                s.status_message
                    .clone()
                    .unwrap_or_else(|| "Error".to_string())
            })
        })
        .style(move |s| {
            s.color(config.get().color(LapceColor::PROBLEM))
                .font_size(12.0)
        }),
    )
    .style(move |s| {
        let is_visible = state.with(|s| {
            matches!(
                s.lifecycle,
                PanelLifecycle::Error { .. } | PanelLifecycle::AuthRequired { .. }
            )
        });
        s.width_full()
            .padding(8.0)
            .background(config.get().color(LapceColor::EDITOR_CURRENT_LINE))
            .apply_if(!is_visible, |s| s.hide())
    });

    // Input Composer Area
    let composer = h_stack((
        container(
            label(move || {
                state.with(|s| {
                    if s.input_draft.is_empty() {
                        "Ask Antares agent...".to_string()
                    } else {
                        s.input_draft.clone()
                    }
                })
            })
            .style(move |s| {
                let is_empty = state.with(|s| s.input_draft.is_empty());
                let color = if is_empty {
                    config.get().color(LapceColor::PANEL_FOREGROUND_DIM)
                } else {
                    config.get().color(LapceColor::PANEL_FOREGROUND)
                };
                s.color(color).width_full()
            }),
        )
        .style(move |s| {
            s.flex_grow(1.0f32)
                .height(32.0)
                .padding_horiz(8.0)
                .items_center()
                .border(1.0)
                .border_radius(4.0)
                .border_color(config.get().color(LapceColor::LAPCE_BORDER))
                .background(config.get().color(LapceColor::EDITOR_BACKGROUND))
        }),
        // Send Button
        clickable_icon(
            || LapceIcons::START,
            {
                let data = data.clone();
                move || {
                    data.send_prompt();
                }
            },
            || false,
            move || !state.with(|s| s.can_send()),
            || "Send Prompt",
            config,
        ),
        // Cancel Button (Active when streaming/cancelling)
        clickable_icon(
            || LapceIcons::CLOSE,
            {
                let data = data.clone();
                move || {
                    data.cancel();
                }
            },
            || false,
            move || !state.with(|s| s.can_cancel()),
            || "Cancel Turn",
            config,
        ),
    ))
    .style(move |s| {
        s.width_full()
            .padding(8.0)
            .gap(6.0, 0.0)
            .items_center()
            .background(config.get().color(LapceColor::EDITOR_BACKGROUND))
            .border_top(1.0)
            .border_color(config.get().color(LapceColor::LAPCE_BORDER))
    });

    // Root Panel Layout
    stack((header, empty_state, transcript, banner, composer)).style(move |s| {
        s.size_full()
            .flex_col()
            .background(config.get().color(LapceColor::PANEL_BACKGROUND))
    })
}
