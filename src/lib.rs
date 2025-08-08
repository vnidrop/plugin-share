//! # tauri-plugin-vnidrop-share
//!
//! A Tauri plugin to share content using the native sharing dialog on Windows, macOS, and mobile platforms.
//!
//! On desktop, the plugin handles sharing text, URLs, and files. For files, it manages their lifecycle by creating
//! temporary files from Base64 content and cleaning them up once the share dialog is closed or the application exits.
//!
//! ## Installation
//!
//! ```sh
//! # Cargo.toml
//! [dependencies]
//! tauri-plugin-vnidrop-share = { git = "[https://github.com/vnidrop/plugin-share](https://github.com/vnidrop/plugin-share)" }
//! ```
//!
//! ## Usage
//!
//! ### Rust
//!
//! You need to initialize the plugin in your `main.rs` or `lib.rs` to register the commands and set up state management.
//!
//! ```rust
//! // src/main.rs
//! fn main() {
//!     tauri::Builder::default()
//!         .plugin(tauri_plugin_vnidrop_share::init())
//!         .run(tauri::generate_context!())
//!         .expect("error while running tauri application");
//! }
//! ```
//!
//! ### Frontend (JavaScript/TypeScript)
//!
//! The plugin provides a JavaScript API to call the commands.
//!
//! ```js
//! import { share, canShare } from '@vnidrop/tauri-plugin-share';
//!
//! // Check if sharing is available
//! const canShareResult = await canShare();
//! console.log(`Can share on this platform: ${canShareResult}`);
//!
//! // Share text and a URL
//! if (canShareResult) {
//!   await share({
//!     title: 'Check this out!',
//!     text: 'I found this cool project built with Tauri.',
//!     url: '[https://tauri.app](https://tauri.app)',
//!   });
//! }
//!
//! // Share a file from Base64 content
//! // The file will be created as a temporary file and cleaned up automatically.
//! const fileContent = '...'; // Your Base64-encoded file data
//! const blob = new Blob([fileContent], { type: 'text/plain' });
//! await share({
//!   files: [new File([blob], 'document.txt')],
//! });
//! ```
//!

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
mod platform;
mod state;

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
///
/// This function sets up the plugin, registers its commands, and configures the
/// state management for temporary files. The cleanup of these files is
/// automatically handled when the application exits.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("vnidrop-share")
        .invoke_handler(tauri::generate_handler![
            commands::share,
            commands::can_share,
            commands::cleanup,
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let share = mobile::init(app, api)?;
            #[cfg(desktop)]
            let share = desktop::init(app, api)?;
            app.manage(share);
            app.manage(state::PluginTempFileManager::new());
            Ok(())
        })
        .on_drop(|app| {
            app.state::<state::PluginTempFileManager>()
                .cleanup_all_managed_files();
        })
        .build()
}
