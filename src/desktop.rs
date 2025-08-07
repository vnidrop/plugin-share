use tauri::{AppHandle, Runtime, Window};
use tauri::plugin::PluginApi;
use crate::{models::*, Result};

use crate::platform;

pub struct Share<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Share<R> {

  pub fn share(&self, window: Window<R>, options: ShareOptions) -> Result<()> {
      platform::share(window, options)
  }

    pub fn can_share(&self, window: Window<R>) -> Result<()> {
        platform::can_share(window)
    }

  pub fn cleanup(&self) -> Result<()> {
      platform::cleanup()
  }
}

pub fn init<R: Runtime, C: serde::de::DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Share<R>> {
    Ok(Share(app.clone()))
}