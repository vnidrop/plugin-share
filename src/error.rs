use raw_window_handle::HandleError;
use serde::Serialize;
use std::sync::mpsc::RecvError;
use tempfile::PathPersistError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Defines the custom error types for the plugin.
///
/// This enum is serializable, allowing these errors to be sent
/// from the Rust backend to the JavaScript frontend.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("Failed to interact with native sharing API: {0}")]
    NativeApi(String),
    #[error("Temporary file operation failed: {0}")]
    TempFile(String),
    #[error("Tauri API error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("Failed to receive from channel: {0}")]
    Recv(#[from] RecvError),
    #[error("File persistence error: {0}")]
    FilePersist(String),
    #[error("Failed to get window handle: {0}")]
    Handle(#[from] HandleError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl From<PathPersistError> for Error {
    fn from(err: PathPersistError) -> Self {
        Error::FilePersist(format!("Failed to persist temporary file: {}", err))
    }
}
