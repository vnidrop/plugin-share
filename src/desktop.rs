use crate::state::PluginTempFileManager;
use crate::{models::*, Result};
use tauri::plugin::PluginApi;
use tauri::{AppHandle, Manager, Runtime, State, Window};

use crate::platform;

/// A handle to the `tauri-plugin-share` APIs for desktop.
///
/// This struct provides the public interface for the plugin's commands,
/// abstracting away the platform-specific implementations.
pub struct Share<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Share<R> {
    pub fn share(
        &self,
        window: Window<R>,
        options: ShareOptions,
        state: State<'_, PluginTempFileManager>,
    ) -> Result<()> {
        platform::share(window, options, state)
    }

    pub fn can_share(&self) -> Result<CanShareResult> {
        platform::can_share()
    }

    pub fn cleanup(&self) -> Result<()> {
        self.0.state::<PluginTempFileManager>().cleanup_all_managed_files();
        platform::cleanup()
    }
}

pub fn init<R: Runtime, C: serde::de::DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Share<R>> {
    Ok(Share(app.clone()))
}
