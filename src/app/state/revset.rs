pub struct RevsetEntry {
    pub name: &'static str,
    pub description: &'static str,
}

pub struct RevsetCategory {
    pub name: &'static str,
    pub entries: Vec<RevsetEntry>,
}

#[must_use]
pub fn get_revset_reference() -> Vec<RevsetCategory> {
    vec![
        RevsetCategory {
            name: "Operators",
            entries: vec![
                RevsetEntry {
                    name: "x-",
                    description: "Parents of x",
                },
                RevsetEntry {
                    name: "x+",
                    description: "Children of x",
                },
                RevsetEntry {
                    name: "::x",
                    description: "Ancestors of x",
                },
                RevsetEntry {
                    name: "x::",
                    description: "Descendants of x",
                },
                RevsetEntry {
                    name: "x::y",
                    description: "Descendants of x that are ancestors of y",
                },
                RevsetEntry {
                    name: "x..y",
                    description: "Ancestors of y not ancestors of x",
                },
                RevsetEntry {
                    name: "x & y",
                    description: "Intersection (both)",
                },
                RevsetEntry {
                    name: "x | y",
                    description: "Union (either)",
                },
                RevsetEntry {
                    name: "~x",
                    description: "Complement (not in x)",
                },
                RevsetEntry {
                    name: "x ~ y",
                    description: "Difference (x but not y)",
                },
            ],
        },
        RevsetCategory {
            name: "Scope & Identity",
            entries: vec![
                RevsetEntry {
                    name: "all()",
                    description: "All visible commits",
                },
                RevsetEntry {
                    name: "none()",
                    description: "Empty set",
                },
                RevsetEntry {
                    name: "root()",
                    description: "Oldest ancestor of all commits",
                },
                RevsetEntry {
                    name: "@",
                    description: "Working copy commit",
                },
                RevsetEntry {
                    name: "mine()",
                    description: "Your authored commits",
                },
                RevsetEntry {
                    name: "trunk()",
                    description: "Default bookmark on remote",
                },
                RevsetEntry {
                    name: "mutable()",
                    description: "Mutable commits",
                },
                RevsetEntry {
                    name: "immutable()",
                    description: "Immutable commits",
                },
                RevsetEntry {
                    name: "working_copies()",
                    description: "Working copies across workspaces",
                },
                RevsetEntry {
                    name: "visible_heads()",
                    description: "All visible head commits",
                },
            ],
        },
        RevsetCategory {
            name: "Bookmarks & Tags",
            entries: vec![
                RevsetEntry {
                    name: "bookmarks([p])",
                    description: "Local bookmark targets",
                },
                RevsetEntry {
                    name: "remote_bookmarks()",
                    description: "Remote bookmark targets",
                },
                RevsetEntry {
                    name: "tracked_remote_bookmarks()",
                    description: "Tracked remote bookmarks",
                },
                RevsetEntry {
                    name: "untracked_remote_bookmarks()",
                    description: "Untracked remote bookmarks",
                },
                RevsetEntry {
                    name: "tags([p])",
                    description: "Tag targets",
                },
                RevsetEntry {
                    name: "remote_tags()",
                    description: "Remote tag targets",
                },
            ],
        },
        RevsetCategory {
            name: "Ancestry & DAG",
            entries: vec![
                RevsetEntry {
                    name: "parents(x, [d])",
                    description: "Parents of x (optional depth)",
                },
                RevsetEntry {
                    name: "children(x, [d])",
                    description: "Children of x (optional depth)",
                },
                RevsetEntry {
                    name: "ancestors(x, [d])",
                    description: "Ancestors of x",
                },
                RevsetEntry {
                    name: "descendants(x, [d])",
                    description: "Descendants of x",
                },
                RevsetEntry {
                    name: "heads(x)",
                    description: "Commits with no descendants in x",
                },
                RevsetEntry {
                    name: "roots(x)",
                    description: "Commits with no ancestors in x",
                },
                RevsetEntry {
                    name: "connected(x)",
                    description: "x::x – fill in gaps",
                },
                RevsetEntry {
                    name: "reachable(s, d)",
                    description: "Reachable from s within domain d",
                },
                RevsetEntry {
                    name: "fork_point(x)",
                    description: "Common ancestor(s) of x",
                },
                RevsetEntry {
                    name: "first_parent(x)",
                    description: "First parent only (for merges)",
                },
                RevsetEntry {
                    name: "first_ancestors(x)",
                    description: "Ancestors via first parent only",
                },
            ],
        },
        RevsetCategory {
            name: "Search & Metadata",
            entries: vec![
                RevsetEntry {
                    name: "description(p)",
                    description: "Match commit description",
                },
                RevsetEntry {
                    name: "subject(p)",
                    description: "Match first line of description",
                },
                RevsetEntry {
                    name: "author(p)",
                    description: "Match author name or email",
                },
                RevsetEntry {
                    name: "author_date(p)",
                    description: "Match author date",
                },
                RevsetEntry {
                    name: "committer(p)",
                    description: "Match committer name or email",
                },
                RevsetEntry {
                    name: "committer_date(p)",
                    description: "Match committer date",
                },
                RevsetEntry {
                    name: "files(expr)",
                    description: "Commits modifying matching paths",
                },
                RevsetEntry {
                    name: "diff_lines(t, [f])",
                    description: "Commits with matching diff text",
                },
            ],
        },
        RevsetCategory {
            name: "State & Filters",
            entries: vec![
                RevsetEntry {
                    name: "conflicts()",
                    description: "Commits with conflicts",
                },
                RevsetEntry {
                    name: "divergent()",
                    description: "Divergent commits",
                },
                RevsetEntry {
                    name: "empty()",
                    description: "Commits modifying no files",
                },
                RevsetEntry {
                    name: "merges()",
                    description: "Merge commits",
                },
                RevsetEntry {
                    name: "signed()",
                    description: "Cryptographically signed",
                },
                RevsetEntry {
                    name: "latest(x, [n])",
                    description: "Latest n commits by date",
                },
                RevsetEntry {
                    name: "present(x)",
                    description: "x, or none() if missing",
                },
                RevsetEntry {
                    name: "exactly(x, n)",
                    description: "x if exactly n commits",
                },
            ],
        },
        RevsetCategory {
            name: "String Patterns",
            entries: vec![
                RevsetEntry {
                    name: "exact:\"str\"",
                    description: "Exact string match",
                },
                RevsetEntry {
                    name: "glob:\"pat\"",
                    description: "Unix-style wildcard (default)",
                },
                RevsetEntry {
                    name: "regex:\"pat\"",
                    description: "Regular expression match",
                },
                RevsetEntry {
                    name: "substring:\"str\"",
                    description: "Substring match",
                },
                RevsetEntry {
                    name: "-i suffix",
                    description: "Case-insensitive (e.g. glob-i:)",
                },
            ],
        },
        RevsetCategory {
            name: "Date Patterns",
            entries: vec![
                RevsetEntry {
                    name: "after:\"date\"",
                    description: "At or after the given date",
                },
                RevsetEntry {
                    name: "before:\"date\"",
                    description: "Before the given date",
                },
                RevsetEntry {
                    name: "\"2 days ago\"",
                    description: "Relative date example",
                },
                RevsetEntry {
                    name: "\"yesterday 5pm\"",
                    description: "Natural language date",
                },
            ],
        },
    ]
}
