use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZelligError {
    #[error("translation failed: {0}")]
    TranslationError(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("AI provider error: {0}")]
    AiError(String),

    #[error("model error: {0}")]
    ModelError(String),

    #[error("download error: {0}")]
    DownloadError(String),

    #[error("language detection error: {0}")]
    DetectionError(String),
}

pub type Result<T> = std::result::Result<T, ZelligError>;

impl From<figment2::Error> for ZelligError {
    fn from(err: figment2::Error) -> Self {
        ZelligError::ConfigError(err.to_string())
    }
}

impl From<genai::Error> for ZelligError {
    fn from(err: genai::Error) -> Self {
        ZelligError::AiError(err.to_string())
    }
}
