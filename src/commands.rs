use tauri::{command, Runtime, Window};

use crate::{error, models};
use crate::desktop;

#[command]
async fn share_text<R: Runtime>(
    window: Window<R>,
    options: models::ShareTextOptions,
) -> Result<(), error::Error> {
    desktop::share_text(window, options)
}

#[command]
async fn share_data<R: Runtime>(
    window: Window<R>,
    options: models::ShareDataOptions,
) -> Result<(), error::Error> {
    desktop::share_data(window, options)
}

#[command]
async fn share_file<R: Runtime>(
    window: Window<R>,
    options: models::ShareFileOptions,
) -> Result<(), error::Error> {
    desktop::share_file(window, options)
}

#[command]
async fn cleanup() -> Result<(), error::Error> {
    desktop::cleanup()
}
