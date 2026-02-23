#[derive(Debug, Clone, PartialEq)]
pub struct HeaderState {
    pub repo_text: String,
    pub branch_text: String,
    pub stats_text: String,
    pub wc_text: String,
    pub op_text: String,
}

impl Default for HeaderState {
    fn default() -> Self {
        Self {
            repo_text: " no repo ".to_string(),
            branch_text: " (detached) ".to_string(),
            stats_text: String::new(),
            wc_text: " Loading... ".to_string(),
            op_text: " OP: ........ ".to_string(),
        }
    }
}
