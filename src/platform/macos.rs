use crate::models::{ShareDataOptions, ShareFileOptions, ShareTextOptions};
use crate::Error;
use base64::{engine::general_purpose, Engine as _};
use objc2::{
    rc::{autoreleasepool, Retained},
    runtime::NSObject,
};
use objc2_app_kit::{NSSharingServicePicker, NSView};
use objc2_foundation::{NSArray, NSRect, NSString, NSURL};
use objc2_core_foundation::geometry::{CGPoint, CGRect, CGSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle};
use std::{
    io::Write,
    path::Path,
    sync::mpsc,
};
use tauri::{Runtime, Window};
use tempfile::{Builder, NamedTempFile};

use objc2::AnyThread;

pub fn share_text<R: Runtime>(
    window: Window<R>,
    options: ShareTextOptions,
) -> Result<(), Error> {
    let text = NSString::from_str(&options.text);
    let items = unsafe {
        NSArray::from_slice(&[text])
    };
    show_share_sheet(window, items)
}

pub fn share_data<R: Runtime>(
    window: Window<R>,
    options: ShareDataOptions,
) -> Result<(), Error> {
    let temp_file = create_temp_file_for_data(&options)?;
    let path_str = temp_file.path().to_string_lossy().to_string();

    // The `NSURL` is immediately used and not stored, ensuring its lifetime is managed correctly
    let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&path_str)) };
    let items = unsafe {
        NSArray::from_slice(&[url])
    };

    // The temporary file will be deleted when `temp_file` goes out of scope
    // after the share sheet is closed.
    show_share_sheet(window, items)
}

pub fn share_file<R: Runtime>(
    window: Window<R>,
    options: ShareFileOptions,
) -> Result<(), Error> {
    // Security: Ensure the file exists.
    if!Path::new(&options.path).exists() {
        return Err(Error::InvalidArgs(format!(
            "File does not exist at path: {}",
            options.path
        )));
    }

    let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&options.path)) };
    let items = autoreleasepool(|_pool| {
        // SAFETY: We are creating a file URL from a path provided by the user.
        // The existence of the file has been checked. The `NSURL` is immediately used.
        NSArray::from_slice(&[url])
    });

    show_share_sheet(window, items)
}

pub fn cleanup() -> Result<(), Error> {
    // Temporary file management on macOS is automatic thanks to `NamedTempFile`.
    // This function can remain empty or perform a more thorough cleanup if needed.
    Ok(())
}

/// Displays the native macOS share sheet.
/// Ensures all UI operations are executed on the main thread and propagates errors correctly.
fn show_share_sheet<R: Runtime>(
    window: Window<R>,
    items: Retained<NSArray<NSObject>>,
) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel();

    window.run_on_main_thread(move |

| {
        let result = (|| -> Result<(), Error> {
            let ns_view = get_ns_view(&window)?;

            autoreleasepool(|_pool| {
                let alloc_picker = unsafe { NSSharingServicePicker::alloc() };
                let picker = unsafe { NSSharingServicePicker::initWithItems(alloc_picker, &items)};

                let bounds =  ns_view.bounds();

                unsafe {
                    picker.showRelativeToRect_ofView_preferredEdge(
                        CGRect {
                            origin: CGPoint {
                                x: bounds.size.width / 2.0,
                                y: bounds.size.height / 2.0,
                            },
                            size: CGSize {
                                width: 0.0,
                                height: 0.0,
                            },
                        },
                        &ns_view,
                        objc2_foundation::NSRectEdge::NSMinYEdge,
                    );
                }
            });
            Ok(())
        })();

        // Send the result back to the calling thread. Panicking here is acceptable
        // because if the receiver has been dropped, the application is in an
        // unrecoverable state where the result of the operation cannot be processed.
        tx.send(result).expect("Failed to send result from main thread");
    })?;

    // Block until the main thread sends a result.
    // The first `?` handles a `RecvError` (if the channel disconnects).
    // The second `?` unwraps the `Result<(), Error>` sent from the closure.
    rx.recv()??;

    Ok(())
}

/// Retrieves the native `NSView` pointer from the Tauri window, compatible with `raw-window-handle` v0.6.
fn get_ns_view<R: Runtime>(window: &Window<R>) -> Result<Retained<NSView>, Error> {
    // `window.window_handle()` returns a `Result<WindowHandle<'_>, _>`.
    // The `WindowHandle` is a lifetime-bound guard that ensures the handle is valid.
    let window_handle: WindowHandle<'_> = window.window_handle()?;
    
    if let RawWindowHandle::AppKit(handle) = window_handle.as_raw() {
        // `handle.ns_view` is a `NonNull<c_void>`, guaranteeing it's not null.
        let ns_view_ptr = handle.ns_view.as_ptr();
        
        // SAFETY: The `raw-window-handle` crate's safety contract guarantees that for the lifetime
        // of the `WindowHandle` guard, this is a valid pointer to an `NSView`. We `retain` it to
        // create a new `Id<NSView>`, which is an owned reference that we can safely use.
        let ns_view: Retained<NSView> = unsafe { Retained::retain(ns_view_ptr.cast()) }.unwrap();
        Ok(ns_view)
    } else {
        Err(Error::NativeApi(
            "Unsupported window handle type on macOS.".to_string(),
        ))
    }
}

/// Creates a secure temporary file from Base64 data.
fn create_temp_file_for_data(options: &ShareDataOptions) -> Result<NamedTempFile, Error> {
    let decoded_bytes = general_purpose::STANDARD
       .decode(&options.data)
       .map_err(|_| Error::InvalidArgs("Invalid Base64 data.".to_string()))?;

    // Security: Sanitize the filename to prevent path traversal attacks.
    let sanitized_name = Path::new(&options.name)
       .file_name()
       .ok_or_else(|| Error::InvalidArgs("Invalid file name.".to_string()))?
       .to_str()
       .ok_or_else(|| Error::InvalidArgs("File name contains invalid UTF-8 characters.".to_string()))?;

    let temp_dir = std::env::temp_dir();

    // Use `tempfile` for secure and unique temporary file creation.
    let mut temp_file = Builder::new()
       .prefix(&format!("{}-", uuid::Uuid::new_v4()))
       .suffix(&format!("-{}", sanitized_name))
       .tempfile_in(temp_dir)
       .map_err(|e| Error::TempFile(format!("Failed to create temp file: {}", e)))?;

    temp_file
       .write_all(&decoded_bytes)
       .map_err(|e| Error::TempFile(format!("Failed to write to temp file: {}", e)))?;

    Ok(temp_file)
}