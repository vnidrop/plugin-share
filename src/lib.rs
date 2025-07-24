use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Share;
#[cfg(mobile)]
use mobile::Share;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the share APIs.
pub trait ShareExt<R: Runtime> {
  fn share(&self) -> &Share<R>;
}

impl<R: Runtime, T: Manager<R>> crate::ShareExt<R> for T {
  fn share(&self) -> &Share<R> {
    self.state::<Share<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("share")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let share = mobile::init(app, api)?;
      #[cfg(desktop)]
      let share = desktop::init(app, api)?;
      app.manage(share);
      Ok(())
    })
    .build()
}
