use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime, Window,
    State
};
use crate::state::PluginTempFileManager;

use crate::{models::*, Result};

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "plugin.vnidrop.share";

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_share);

/// Initializes the mobile platform implementation of the plugin.
///
/// This function registers the mobile plugin and returns a handle to its APIs.
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<Share<R>> {
    #[cfg(target_os = "android")]
    let handle = api
        .register_android_plugin(PLUGIN_IDENTIFIER, "SharePlugin")
        .unwrap();
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_share)?;
    Ok(Share(handle))
}

/// A handle to the `tauri-plugin-share` APIs for mobile.
///
/// This struct provides the public interface for the plugin's commands on mobile,
/// which are invoked through the mobile bridge.
pub struct Share<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Share<R> {
    pub fn share(&self, _window: Window<R>, payload: ShareOptions, _state: State<'_, PluginTempFileManager>) -> Result<()> {
        self.0
            .run_mobile_plugin("share", payload)
            .map_err(Into::into)
    }

    pub fn can_share(&self) -> Result<CanShareResult> {
        self.0.run_mobile_plugin("canShare", ()).map_err(Into::into)
    }

    pub fn cleanup(&self) -> Result<()> {
        self.0.run_mobile_plugin("cleanup", ()).map_err(Into::into)
    }
}
