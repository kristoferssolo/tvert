/// The export operation requested by the user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportAction {
    Copy,
    Save,
}
