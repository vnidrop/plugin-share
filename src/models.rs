use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareTextOptions {
    pub text: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareDataOptions {
    pub data: String,
    pub name: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareFileOptions {
  /// The absolute path to the file.
    pub path: String,
    pub title: Option<String>,
}

/// Represents a file that can be shared, including its Base64 content, name, and MIME type.
/// The `data` field contains the Base64 encoded content of the file.
/// The `name` field is the original name of the file, and `mime_type` is the MIME type of the file.
/// This struct is used to share files through the plugin, allowing for easy serialization and deserialization.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedFile {
    pub data: String, 
    pub name: String,
    pub mime_type: String,
}

/// Represents the options for sharing content, including text, title, URL, and files.
/// The `text` field is optional and can be used to provide additional information.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareOptions {
    pub text: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub files: Option<Vec<SharedFile>>,
}
