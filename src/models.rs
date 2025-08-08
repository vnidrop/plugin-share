use serde::{Deserialize, Serialize};

/// Represents a file to be shared, including its content, name, and MIME type.
///
/// The `data` field holds the Base64 encoded content of the file. This approach
/// allows files to be easily passed from the frontend to the Rust backend
/// without needing to manage local file paths directly.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SharedFile {
    pub data: String,
    pub name: String,
    pub mime_type: String,
}

/// Defines the content and options for a native sharing dialog.
///
/// This struct can be used to share text, a title, a URL, and a list of files.
/// All fields are optional, allowing for flexible sharing payloads.
///
/// ## Examples
///
/// To share a simple message and URL:
///
/// ```json
/// {
///   "title": "My Tauri App",
///   "text": "Check out this great app built with Tauri!",
///   "url": "[https://tauri.app](https://tauri.app)"
/// }
/// ```
///
/// To share a file (e.g., an image in Base64 format):
///
/// ```json
/// {
///   "files": [
///     {
///       "data": "data:image/png;base64,iVBORw0KGgo...",
///       "name": "my-image.png",
///       "mimeType": "image/png"
///     }
///   ]
/// }
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ShareOptions {
    /// Optional text content to include in the share dialog.
    pub text: Option<String>,
    /// Optional title for the share dialog. (This is mainly used on Android)
    pub title: Option<String>,
    /// Optional URL to include in the share dialog.
    pub url: Option<String>,
    /// A list of files to share, each represented by a `SharedFile` struct.
    pub files: Option<Vec<SharedFile>>,
}

/// The result type for the `can_share` command.
///
/// A `true` value indicates that the current platform supports native sharing.
/// The [`crate::commands::can_share`] command will return `true` on Windows, macOS, and mobile platforms,
/// and `false` on Linux since there is no native sharing dialog available.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CanShareResult {
    pub value: bool,
}
