#[derive(Debug, Clone, PartialEq, Default)]
pub struct EvologState {
    pub content: Vec<String>,
    pub scroll: u16,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OperationLogState {
    pub content: Vec<String>,
    pub scroll: u16,
}
