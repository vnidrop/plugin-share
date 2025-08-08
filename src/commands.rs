use tauri::{command, AppHandle, Runtime, State, Window};

use crate::{error, models, state::PluginTempFileManager, ShareExt};

#[command]
pub async fn share<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    options: models::ShareOptions,
    state: State<'_, PluginTempFileManager>,
) -> Result<(), error::Error> {
    app.share().share(window, options, state)
}

#[command]
pub async fn can_share<R: Runtime>(
    app: AppHandle<R>,
) -> Result<models::CanShareResult, error::Error> {
    app.share().can_share()
}

#[command]
pub async fn cleanup<R: Runtime>(app: AppHandle<R>) -> Result<(), error::Error> {
    app.share().cleanup()
}
