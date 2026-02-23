use super::types::{FooterGroup, FooterItem};
use crate::app::state::{AppMode, AppState};

pub fn get_groups(state: &AppState) -> Vec<FooterGroup> {
    if state.last_error.is_some() {
        return vec![FooterGroup {
            name: "ERROR",
            items: vec![FooterItem {
                key: "Esc",
                desc: "dismiss",
                highlighted: false,
            }],
        }];
    }

    let is_conflict = state.is_selected_file_conflicted();

    match state.mode {
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
                    FooterItem {
                        key: "C",
                        desc: "clear",
                        highlighted: state.revset.is_some(),
                    },
                ],
            });

            if state.show_diffs {
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
        AppMode::Input
        | AppMode::BookmarkInput
        | AppMode::CommitInput
        | AppMode::FilterInput
        | AppMode::RebaseInput => vec![FooterGroup {
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
        AppMode::RebaseSelect => vec![FooterGroup {
            name: "REBASE",
            items: vec![
                FooterItem {
                    key: "j/k",
                    desc: "select target",
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
        AppMode::Evolog => vec![FooterGroup {
            name: "EVOLOG",
            items: vec![
                FooterItem {
                    key: "j/k",
                    desc: "scroll",
                    highlighted: false,
                },
                FooterItem {
                    key: "q/Esc",
                    desc: "close",
                    highlighted: false,
                },
            ],
        }],
        AppMode::OperationLog => vec![FooterGroup {
            name: "OP LOG",
            items: vec![
                FooterItem {
                    key: "j/k",
                    desc: "scroll",
                    highlighted: false,
                },
                FooterItem {
                    key: "q/Esc",
                    desc: "close",
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
