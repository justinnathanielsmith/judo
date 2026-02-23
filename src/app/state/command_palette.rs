#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommandPaletteState {
    pub query: String,
    pub matches: Vec<usize>, // Indices into predefined command list
    pub selected_index: usize,
}
