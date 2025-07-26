use tauri::{command, AppHandle, Runtime, Window};

use crate::{error, models, ShareExt};

#[command]
pub async fn share_text<R: Runtime>(
    app: AppHandle<R>,
    window: Window<R>,
    options: models::ShareTextOptions,
) -> Result<(), error::Error> {
    app.share().share_text(window, options)
}

#[command]
pub async fn share_data<R: Runtime>(
    app: AppHandle<R>, 
    window: Window<R>,
    options: models::ShareDataOptions,
) -> Result<(), error::Error> {
    app.share().share_data(window, options)
}

#[command]
pub async fn share_file<R: Runtime>(
    app: AppHandle<R>, 
    window: Window<R>,
    options: models::ShareFileOptions,
) -> Result<(), error::Error> {
    app.share().share_file(window, options)
}

#[command]
async fn cleanup<R: Runtime>(app: AppHandle<R>) -> Result<(), error::Error> {
    app.share().cleanup()
}
