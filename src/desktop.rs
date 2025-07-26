use tauri::{AppHandle, Runtime, Window};
use tauri::plugin::PluginApi;
use crate::{models::*, error::Error};

use crate::platform;

pub struct Share<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Share<R> {

  pub fn share_text(&self, window: Window<R>, options: ShareTextOptions) -> Result<(), Error> {
      platform::share_text(window, options)
  }

  pub fn share_data(&self, window: Window<R>, options: ShareDataOptions) -> Result<(), Error> {
      platform::share_data(window, options)
  }

  pub fn share_file(&self, window: Window<R>, options: ShareFileOptions) -> Result<(), Error> {
      platform::share_file(window, options)
  }

  pub fn cleanup(&self) -> Result<(), Error> {
      platform::cleanup()
  }
}

pub fn init<R: Runtime, C: serde::de::DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Share<R>> {
    Ok(Share(app.clone()))
}