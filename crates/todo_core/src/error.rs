use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    InvalidInput(String),
    InvalidData(String),
    Io(String),
}

impl AppError {
    pub fn invalid_input<M: Into<String>>(message: M) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn invalid_data<M: Into<String>>(message: M) -> Self {
        Self::InvalidData(message.into())
    }

    pub fn io<M: Into<String>>(message: M) -> Self {
        Self::Io(message.into())
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::InvalidData(_) => "invalid_data",
            Self::Io(_) => "io_error",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::InvalidInput(message) => message,
            Self::InvalidData(message) => message,
            Self::Io(message) => message,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.code(), self.message())
    }
}

impl std::error::Error for AppError {}
