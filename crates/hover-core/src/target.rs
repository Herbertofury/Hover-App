use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    LocalFile,
    BrowserRemote,
    ShellItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreviewSource {
    pub kind: SourceKind,
    pub path: Option<PathBuf>,
    pub url: Option<String>,
    pub modified_ticks: Option<u64>,
    pub generation_hint: u64,
}

impl PreviewSource {
    pub fn local_file(path: PathBuf, modified_ticks: Option<u64>, generation_hint: u64) -> Self {
        Self {
            kind: SourceKind::LocalFile,
            path: Some(path),
            url: None,
            modified_ticks,
            generation_hint,
        }
    }
}
