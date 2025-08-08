use tauri::{Runtime, State, Window};

use crate::models::{CanShareResult, ShareOptions};
use crate::state::PluginTempFileManager;
use crate::Error;

pub fn share<R: Runtime>(
    _window: Window<R>,
    _options: ShareOptions,
    _state: State<'_, PluginTempFileManager>,
) -> Result<(), Error> {
    Ok(())
}

pub fn can_share() -> Result<CanShareResult, Error> {
    Ok(CanShareResult { value: false })
}

pub fn cleanup() -> Result<(), Error> {
    Ok(())
}
