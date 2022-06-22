#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("multipart error: {0}")]
    Multipart(#[from] multer::Error),
    #[error("expected {0:?} field")]
    ExpectedField(String),
    #[error("unexpected end of fields")]
    UnexpectedEnd,
    #[error("could not parse to {target}: {error}")]
    Custom { target: &'static str, error: String },
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
