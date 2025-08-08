use tauri::{command, AppHandle, Runtime, State, Window};

use crate::{error, models, state::PluginTempFileManager, ShareExt};

/// The main command to share content.
///
/// This command accepts a `ShareOptions` struct containing the content to be shared.
/// It creates and displays the native sharing dialog for the current platform.
///
/// The temporary files created for sharing will be automatically managed and
/// cleaned up.
///
/// ## Arguments
///
/// * `app`: The Tauri application handle.
/// * `window`: The Tauri window from which the sharing dialog will be shown.
/// * `options`: A `ShareOptions` struct defining the content to share.
/// * `state`: The `PluginTempFileManager` state, used internally to manage file cleanup.
///
/// ## Example
///
/// See the plugin's frontend usage example in the `lib.rs` file.
#[command]
pub async fn share<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    options: models::ShareOptions,
    state: State<'_, PluginTempFileManager>,
) -> Result<(), error::Error> {
    app.share().share(window, options, state)
}


/// Checks if the native sharing dialog is available on the current platform.
///
/// This is useful for conditionally showing a share button in the frontend.
/// It returns `true` on Windows, macOS, and mobile, and `false` on Linux.
///
/// ## Arguments
///
/// * `app`: The Tauri application handle.
///
/// ## Returns
///
/// A `CanShareResult` struct containing a boolean value.
#[command]
pub async fn can_share<R: Runtime>(
    app: AppHandle<R>,
) -> Result<models::CanShareResult, error::Error> {
    app.share().can_share()
}

/// Manually triggers the cleanup of temporary files.
///
/// While file cleanup is automatically handled when the app exits, this command
/// can be used to manually force a cleanup, for example, after a file is shared
/// and is no longer needed by the plugin.
///
/// ## Arguments
///
/// * `app`: The Tauri application handle.
#[command]
pub async fn cleanup<R: Runtime>(app: AppHandle<R>) -> Result<(), error::Error> {
    app.share().cleanup()
}
