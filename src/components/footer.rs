use crate::app::state::{AppMode, AppState};
use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct FooterItem {
    pub key: &'static str,
    pub desc: &'static str,
    pub highlighted: bool,
}

pub struct FooterGroup {
    pub name: &'static str,
    pub items: Vec<FooterItem>,
}

pub struct Footer<'a> {
    pub state: &'a AppState<'a>,
    pub theme: &'a Theme,
}

impl Footer<'_> {
    fn get_groups(&self) -> Vec<FooterGroup> {
        if self.state.last_error.is_some() {
            return vec![FooterGroup {
                name: "ERROR",
                items: vec![FooterItem {
                    key: "Esc",
                    desc: "dismiss",
                    highlighted: false,
                }],
            }];
        }

        let is_conflict = self.state.is_selected_file_conflicted();

        match self.state.mode {
            AppMode::Normal => {
                let mut groups = Vec::new();
                groups.push(FooterGroup {
                    name: "NAV",
                    items: vec![
                        FooterItem {
                            key: "j/k",
                            desc: "move",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "/",
                            desc: "filt",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "m/t/c",
                            desc: "mine/trnk/conf",
                            highlighted: false,
                        },
                    ],
                });

                if self.state.show_diffs {
                    groups[0].items.push(FooterItem {
                        key: "Tab",
                        desc: "focus",
                        highlighted: false,
                    });
                    groups.push(FooterGroup {
                        name: "DIFF",
                        items: vec![
                            FooterItem {
                                key: "PgUp/Dn",
                                desc: "scroll",
                                highlighted: false,
                            },
                            FooterItem {
                                key: "[/]",
                                desc: "hunk",
                                highlighted: false,
                            },
                        ],
                    });
                }

                groups.push(FooterGroup {
                    name: "EDIT",
                    items: vec![
                        FooterItem {
                            key: "ENTER",
                            desc: "select",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "d",
                            desc: "desc",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "n",
                            desc: "new",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "e",
                            desc: "edit",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "a",
                            desc: "abdn",
                            highlighted: false,
                        },
                    ],
                });

                groups.push(FooterGroup {
                    name: "VCS",
                    items: vec![
                        FooterItem {
                            key: "s/S",
                            desc: "snap/sqsh",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "f",
                            desc: "fetch",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "p",
                            desc: "push",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "b/B",
                            desc: "bkmk",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "u/U",
                            desc: "undo",
                            highlighted: false,
                        },
                    ],
                });

                groups.push(FooterGroup {
                    name: "APP",
                    items: vec![FooterItem {
                        key: "q",
                        desc: "quit",
                        highlighted: false,
                    }],
                });
                groups
            }
            AppMode::Diff => vec![
                FooterGroup {
                    name: "NAV",
                    items: vec![
                        FooterItem {
                            key: "j/k",
                            desc: "file",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "h/Tab",
                            desc: "back",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "PgUp/Dn",
                            desc: "scroll",
                            highlighted: false,
                        },
                        FooterItem {
                            key: "m/ENTER",
                            desc: "merge",
                            highlighted: is_conflict,
                        },
                    ],
                },
                FooterGroup {
                    name: "APP",
                    items: vec![FooterItem {
                        key: "q",
                        desc: "quit",
                        highlighted: false,
                    }],
                },
            ],
            AppMode::Input | AppMode::BookmarkInput | AppMode::FilterInput => vec![FooterGroup {
                name: "INPUT",
                items: vec![
                    FooterItem {
                        key: "ENTER",
                        desc: "submit",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "Esc",
                        desc: "cancel",
                        highlighted: false,
                    },
                ],
            }],
            AppMode::ContextMenu => vec![FooterGroup {
                name: "MENU",
                items: vec![
                    FooterItem {
                        key: "j/k",
                        desc: "move",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "ENTER",
                        desc: "select",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "Esc",
                        desc: "close",
                        highlighted: false,
                    },
                ],
            }],
            AppMode::SquashSelect => vec![FooterGroup {
                name: "SQUASH",
                items: vec![
                    FooterItem {
                        key: "j/k",
                        desc: "select",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "ENTER",
                        desc: "confirm",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "Esc",
                        desc: "cancel",
                        highlighted: false,
                    },
                ],
            }],
            AppMode::CommandPalette => vec![FooterGroup {
                name: "COMMAND",
                items: vec![
                    FooterItem {
                        key: "ENTER",
                        desc: "run",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "Esc",
                        desc: "cancel",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "j/k",
                        desc: "move",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "ctrl+n/p",
                        desc: "move",
                        highlighted: false,
                    },
                ],
            }],
            AppMode::Loading => vec![FooterGroup {
                name: "LOADING",
                items: vec![],
            }],
            AppMode::Help => vec![FooterGroup {
                name: "HELP",
                items: vec![FooterItem {
                    key: "q/Esc/?",
                    desc: "close",
                    highlighted: false,
                }],
            }],
            AppMode::ThemeSelection => vec![FooterGroup {
                name: "THEME",
                items: vec![
                    FooterItem {
                        key: "j/k",
                        desc: "select",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "ENTER",
                        desc: "apply",
                        highlighted: false,
                    },
                    FooterItem {
                        key: "Esc",
                        desc: "cancel",
                        highlighted: false,
                    },
                ],
            }],
            AppMode::NoRepo => vec![
                FooterGroup {
                    name: "INIT",
                    items: vec![FooterItem {
                        key: "i/ENTER",
                        desc: "initialize",
                        highlighted: false,
                    }],
                },
                FooterGroup {
                    name: "APP",
                    items: vec![FooterItem {
                        key: "q/Esc",
                        desc: "quit",
                        highlighted: false,
                    }],
                },
            ],
        }
    }
}

impl Widget for Footer<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let state = self.state;

        // Status segment
        let status_span = if let Some(err) = &state.last_error {
            Span::styled(format!("  ERROR: {}  ", err.message), theme.status_error)
        } else if let Some(msg) = &state.status_message {
            Span::styled(format!("  {msg}  "), theme.status_info)
        } else {
            Span::styled("  READY  ", theme.status_ready)
        };

        let mut spans = vec![status_span, Span::raw(" ")];

        // Repo context (Workspace, WC & Operation)
        if !state.workspace_id.is_empty() {
            spans.push(Span::styled(
                format!(" {} ", state.workspace_id),
                theme.header_item,
            ));
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(&state.header_state.wc_text, theme.header_item));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(&state.header_state.op_text, theme.header_item));
        spans.push(Span::raw(" "));

        // Background tasks
        if state.active_tasks.is_empty() {
            spans.push(Span::raw("  "));
        } else {
            let tasks_text = format!(
                " {} tasks: {} ",
                state.spinner,
                state.active_tasks.join(", ")
            );
            spans.push(Span::styled(tasks_text, theme.status_info));
            spans.push(Span::raw("  "));
        }

        let groups = self.get_groups();

        let available_width = area.width.saturating_sub(4); // Margin
        let mut current_width = spans
            .iter()
            .map(ratatui::prelude::Span::width)
            .sum::<usize>();

        for group in groups {
            if group.items.is_empty() {
                continue;
            }

            // Check if we can fit at least the first item of the group
            let first_item = &group.items[0];
            let first_item_width = first_item.key.len() + first_item.desc.len() + 4;

            if current_width + first_item_width > available_width as usize {
                break;
            }

            // Add group name as a subtle label if there's plenty of space
            if area.width > 100 {
                let group_label =
                    Span::styled(format!("{}: ", group.name), theme.footer_group_name);
                if current_width + group_label.width() + first_item_width < available_width as usize
                {
                    spans.push(group_label);
                    current_width += group.name.len() + 2;
                }
            }

            for item in group.items {
                let key_str = format!(" {} ", item.key);
                let desc_str = format!(" {} ", item.desc);

                let item_width = key_str.len() + desc_str.len();
                if current_width + item_width + 1 > available_width as usize {
                    break;
                }

                let key_style = if item.highlighted {
                    theme.header_active
                } else {
                    theme.footer_segment_key
                };

                let val_style = if item.highlighted {
                    theme
                        .header_active
                        .add_modifier(ratatui::style::Modifier::DIM)
                } else {
                    theme.footer_segment_val
                };

                spans.push(Span::styled(key_str, key_style));
                spans.push(Span::styled(desc_str, val_style));
                spans.push(Span::raw(" "));
                current_width += item_width + 1;
            }
            spans.push(Span::raw("  "));
            current_width += 2;
        }

        Paragraph::new(Line::from(spans))
            .style(theme.footer)
            .render(area, buf);
    }
}
