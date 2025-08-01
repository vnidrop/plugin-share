use serde::{Serialize};
use thiserror::Error;
use raw_window_handle::HandleError;
use std::sync::mpsc::RecvError;

pub type Result<T> = std::result::Result<T, Error>;

// This enum defines the errors that can be sent back to the frontend.
// Using `thiserror` makes it easy to convert from other error types,
// and `serde::Serialize` allows it to be returned in a command's `Err` variant.
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
    #[error("Failed to get window handle: {0}")]
    Handle(#[from] HandleError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}