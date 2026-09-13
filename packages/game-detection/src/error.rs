use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse {what}: {detail}")]
    Parse { what: &'static str, detail: String },
    #[error("{0}")]
    Rejected(String),
}

pub type Result<T> = std::result::Result<T, DetectError>;

pub fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> DetectError {
    let path = path.into();
    move |source| DetectError::Io { path, source }
}
