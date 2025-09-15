# Tauri Plugin share

`tauri-plugin-vnidrop-share`

A Tauri plugin that provides a seamless, cross-platform interface for native sharing. This plugin allows your application to share text, URLs, and files using the native share dialogs on macOS, Windows, and mobile devices.

## Why this plugin?

The web's native [Web Share API](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/share) offers a great way to integrate with a device's sharing capabilities. However, a key limitation is that it only works in a **secure context** (i.e., HTTPS). Since Tauri applications run in a local, non-secure context, the Web Share API is unavailable. This plugin replicates the functionality of the Web Share API, providing a familiar and easy-to-use interface for Tauri developers while leveraging the underlying native APIs to ensure full functionality on all supported platforms.

For file sharing, the plugin intelligently manages the lifecycle of temporary files. It creates secure temporary files from Base64 data, ensuring they persist for the duration of the sharing operation, and automatically cleans them up when the application exits. On mobile platforms like Android and iOS, the native sharing APIs are directly invoked, and temporary files are managed and cleaned up within the native code.

## Installation

### Rust

Add the plugin to your `Cargo.toml`:

```sh
[dependencies]
tauri-plugin-vnidrop-share = "0.2.0"
```

### Frontend

Install the JavaScript package using npm:

```sh
npm install @vnidrop/tauri-plugin-share
```

## Usage

### Frontend (TypeScript/JavaScript)

The frontend API is designed to closely resemble the Web Share API, making it intuitive for developers.

1. **Checking Share Availability**

   Use the `canShare()` function to check if the current platform supports native sharing. This is useful for conditionally displaying a share button. On Linux, this function will return `false`, and calling `share()` will do nothing.

   ```ts
   import { canShare } from "@vnidrop/tauri-plugin-share";

   async function checkShareSupport() {
     const isShareAvailable = await canShare();
     if (isShareAvailable) {
       console.log("Sharing is supported on this platform!");
       // Show share button
     } else {
       console.log("Sharing is not available.");
       // Hide share button
     }
   }
   ```

2. **Sharing Content**

   Use the `share()` function with a `ShareData` object to trigger the native dialog. The files field requires an array of `File` objects, which the plugin automatically handles by converting them to Base64 and managing their lifecycle in the backend.

   ```ts
   import { share, canShare } from "@vnidrop/tauri-plugin-share";

   // Share text and a URL
   async function shareTextAndUrl() {
     if (await canShare()) {
       await share({
         title: "My Project",
         text: "Check out this awesome project built with Tauri!",
         url: "https://github.com/vnidrop/plugin-share",
       });
       console.log("Share dialog closed.");
     }
   }

   // Share a file (e.g., an image)
   async function shareFile() {
     const fileInput = document.querySelector(
       'input[type="file"]'
     ) as HTMLInputElement;
     const file = fileInput.files?.[0];

     if (file && (await canShare())) {
       await share({
         title: "Shared File",
         files: [file],
       });
       console.log("File shared successfully.");
     }
   }
   ```

3. Manual Cleanup

   While the plugin automatically handles cleanup when the app exits, you can manually call `cleanup()` to remove temporary files immediately after a share operation is complete to free up disk space.

   ```ts
   import { cleanup } from "@vnidrop/tauri-plugin-share";

   await cleanup();
   console.log("Temporary files have been cleaned up.");
   ```

### Rust

1. **Plugin Initialization**

   Add the plugin to your `main.rs` file within the `tauri::Builder` to register the commands and enable state management for file cleanup on application exit.

   ```rs
   // src/main.rs
    fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_vnidrop_share::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    }
   ```

2. **Using the `ShareExt` Trait**

   The `ShareExt` trait is provided for a more idiomatic way to access the plugin's functionalities directly from an `AppHandle` or `Window`.

   ```rs
   // A Tauri command example
    use tauri::{command, AppHandle, Runtime};
    use tauri_plugin_vnidrop_share::{ShareExt, Result, ShareOptions};

    #[command]
    async fn custom_share_command<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    let share_options = ShareOptions {
        text: Some("Hello from Rust!".to_string()),
        title: Some("Rust Share".to_string()),
        url: None,
        files: None,
    };

    // Use the extension trait to access the plugin's API
    app.share().share(app.get_webview_window("main").unwrap(), share_options, app.state())?;
    Ok(())
    }
   ```
