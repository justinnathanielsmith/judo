use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use crate::theme::Theme;
use crate::app::state::{AppMode, AppState};

pub struct FooterItem {
    pub key: &'static str,
    pub desc: &'static str,
}

pub struct FooterGroup {
    pub name: &'static str,
    pub items: Vec<FooterItem>,
}

pub struct Footer<'a> {
    pub state: &'a AppState<'a>,
    pub theme: &'a Theme,
}

impl<'a> Footer<'a> {
    fn get_groups(&self) -> Vec<FooterGroup> {
        if self.state.last_error.is_some() {
            return vec![
                FooterGroup {
                    name: "ERROR",
                    items: vec![
                        FooterItem { key: "Esc", desc: "dismiss" },
                    ],
                },
            ];
        }
        match self.state.mode {
            AppMode::Normal => {
                let mut groups = vec![
                    FooterGroup {
                        name: "NAV",
                        items: vec![
                            FooterItem { key: "j/k", desc: "move" },
                        ],
                    },
                ];

                if self.state.show_diffs {
                    groups[0].items.push(FooterItem { key: "Tab", desc: "focus" });
                    groups.push(FooterGroup {
                        name: "DIFF",
                        items: vec![
                            FooterItem { key: "PgUp/Dn", desc: "scroll" },
                            FooterItem { key: "[/]", desc: "hunk" },
                        ],
                    });
                }

                groups.push(FooterGroup {
                    name: "EDIT",
                    items: vec![
                        FooterItem { key: "ENTER", desc: "select" },
                        FooterItem { key: "d", desc: "desc" },
                        FooterItem { key: "n", desc: "new" },
                        FooterItem { key: "e", desc: "edit" },
                        FooterItem { key: "a", desc: "abdn" },
                    ],
                });

                groups.push(FooterGroup {
                    name: "VCS",
                    items: vec![
                        FooterItem { key: "s/S", desc: "snap/sqsh" },
                        FooterItem { key: "b/B", desc: "bkmk" },
                        FooterItem { key: "u/U", desc: "undo" },
                    ],
                });

                groups.push(FooterGroup {
                    name: "APP",
                    items: vec![
                        FooterItem { key: "q", desc: "quit" },
                    ],
                });
                groups
            }
            AppMode::Diff => vec![
                FooterGroup {
                    name: "NAV",
                    items: vec![
                        FooterItem { key: "j/k", desc: "scroll" },
                        FooterItem { key: "h/Tab", desc: "back" },
                        FooterItem { key: "PgUp/Dn", desc: "page" },
                        FooterItem { key: "[/]", desc: "hunk" },
                    ],
                },
                FooterGroup {
                    name: "APP",
                    items: vec![
                        FooterItem { key: "q", desc: "quit" },
                    ],
                },
            ],
            AppMode::Input | AppMode::BookmarkInput => vec![
                FooterGroup {
                    name: "INPUT",
                    items: vec![
                        FooterItem { key: "ENTER", desc: "submit" },
                        FooterItem { key: "Esc", desc: "cancel" },
                    ],
                },
            ],
            AppMode::ContextMenu => vec![
                FooterGroup {
                    name: "MENU",
                    items: vec![
                        FooterItem { key: "j/k", desc: "move" },
                        FooterItem { key: "ENTER", desc: "select" },
                        FooterItem { key: "Esc", desc: "close" },
                    ],
                },
            ],
            AppMode::SquashSelect => vec![
                FooterGroup {
                    name: "SQUASH",
                    items: vec![
                        FooterItem { key: "j/k", desc: "select" },
                        FooterItem { key: "ENTER", desc: "confirm" },
                        FooterItem { key: "Esc", desc: "cancel" },
                    ],
                },
            ],
            AppMode::Command => vec![
                FooterGroup {
                    name: "COMMAND",
                    items: vec![
                        FooterItem { key: "ENTER", desc: "run" },
                        FooterItem { key: "Esc", desc: "cancel" },
                    ],
                },
            ],
            AppMode::Loading => vec![
                FooterGroup {
                    name: "LOADING",
                    items: vec![],
                },
            ],
        }
    }
}

impl<'a> Widget for Footer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let state = self.state;

        // Status segment
        let status_span = if let Some(err) = &state.last_error {
            Span::styled(format!("  ERROR: {}  ", err), theme.status_error)
        } else if let Some(msg) = &state.status_message {
            Span::styled(format!("  {}  ", msg), theme.status_info)
        } else {
            Span::styled("  READY  ", theme.status_ready)
        };

        let groups = self.get_groups();
        let mut spans = vec![status_span, Span::raw("  ")];

        let available_width = area.width.saturating_sub(4); // Margin
        let mut current_width = spans.iter().map(|s| s.width()).sum::<usize>();

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
                 let group_label = Span::styled(format!("{}: ", group.name), theme.footer_group_name);
                 if current_width + group_label.width() + first_item_width < available_width as usize {
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

                spans.push(Span::styled(key_str, theme.footer_segment_key));
                spans.push(Span::styled(desc_str, theme.footer_segment_val));
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
