use tauri::{command, AppHandle, Runtime, Window};

use crate::{error, models, ShareExt};

#[command]
pub async fn share<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    options: models::ShareOptions,
) -> Result<(), error::Error> {
    app.share().share(window, options)
}

#[command]
pub async fn can_share<R: Runtime>(
    app: AppHandle<R>, 
    window: Window<R>,
) -> Result<(), error::Error> {
    app.share().can_share(window)
}

#[command]
pub async fn cleanup<R: Runtime>(app: AppHandle<R>) -> Result<(), error::Error> {
    app.share().cleanup()
}
