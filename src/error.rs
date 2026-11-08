//! Error type shared by every module.

/// A human-readable error message that is printed to the user as-is.
#[derive(Debug)]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error(e.to_string())
    }
}

/// Builds an `Err` that carries `msg`.
pub fn fail<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error(msg.into()))
}

/// Prefixes an I/O error with a description of the operation that failed.
pub trait Context<T> {
    fn context(self, what: &str) -> Result<T>;
}

impl<T> Context<T> for std::io::Result<T> {
    fn context(self, what: &str) -> Result<T> {
        self.map_err(|e| Error(format!("{what}: {e}")))
    }
}
