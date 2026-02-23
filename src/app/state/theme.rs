#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSelectionState {
    pub selected_index: usize,
    pub themes: Vec<crate::theme::PaletteType>,
}

impl Default for ThemeSelectionState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            themes: crate::theme::PaletteType::all().to_vec(),
        }
    }
}
