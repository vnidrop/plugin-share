use tauri::{Runtime, Window};

use crate::models::{ShareDataOptions, ShareFileOptions, ShareTextOptions};
use crate::Error;

pub fn share_text<R: Runtime>(
    _window: Window<R>,
    _options: ShareTextOptions,
) -> Result<(), Error> {
    Ok(())
}

pub fn share_data<R: Runtime>(
    _window: Window<R>,
    _options: ShareDataOptions,
) -> Result<(), Error> {
    Ok(())
}

pub fn share_file<R: Runtime>(
    _window: Window<R>,
    _options: ShareFileOptions,
) -> Result<(), Error> {
    Ok(())
}

pub fn cleanup() -> Result<(), Error> {
    
    Ok(())
}