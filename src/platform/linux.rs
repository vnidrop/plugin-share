use tauri::{Runtime, Window};

use crate::models::{ShareOptions, CanShareResult};
use crate::Error;

pub fn share<R: Runtime>(
    _window: Window<R>,
    _options: ShareOptions,
) -> Result<(), Error> {
    Ok(())
}

pub fn can_share() -> Result<CanShareResult, Error> {
    Ok(CanShareResult { value: false })
}


pub fn cleanup() -> Result<(), Error> {
    Ok(())
}