use crate::models::{Error, ShareDataOptions, ShareFileOptions, ShareTextOptions};
use base64::{engine::general_purpose, Engine as _};
use objc2::{
    rc::{autoreleasepool, Id},
    ClassType,
};
use objc2_app_kit::{NSSharingServicePicker, NSView};
use objc2_foundation::{NSArray, NSObject, NSRect, NSString, NSURL};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{io::Write, path::Path};
use tauri::{Runtime, Window};
use tempfile::{Builder, NamedTempFile};

pub fn share_text<R: Runtime>(
    window: Window<R>,
    options: ShareTextOptions,
) -> Result<(), Error> {
    let items = autoreleasepool(|pool| {
        let text = NSString::from_str(&options.text);
        // The macOS share sheet intelligently handles URLs within the text.
        // We simply pass the text as the main object.
        NSArray::from_slice(&[Id::into_super(text)])
    });
    show_share_sheet(window, items)
}

pub fn share_data<R: Runtime>(
    window: Window<R>,
    options: ShareDataOptions,
) -> Result<(), Error> {
    let temp_file = create_temp_file_for_data(&options)?;
    let path_str = temp_file.path().to_string_lossy().to_string();

    let items = autoreleasepool(|pool| {
        let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&path_str)) };
        NSArray::from_slice(&[Id::into_super(url)])
    });

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

    let items = autoreleasepool(|pool| {
        let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&options.path)) };
        NSArray::from_slice(&[Id::into_super(url)])
    });

    show_share_sheet(window, items)
}

pub fn cleanup() -> Result<(), Error> {
    // Temporary file management on macOS is automatic thanks to `NamedTempFile`.
    // This function can remain empty or perform a more thorough cleanup if needed.
    Ok(())
}

/// Displays the native macOS share sheet.
/// Ensures all UI operations are executed on the main thread.
fn show_share_sheet<R: Runtime>(
    window: Window<R>,
    items: Id<NSArray<NSObject>>,
) -> Result<(), Error> {
    window.run_on_main_thread(move || {
        let ns_view = get_ns_view(&window)?;

        autoreleasepool(|_pool| {
            let picker = unsafe { NSSharingServicePicker::initWithItems(&items) };

            // We need to show the picker relative to a view and a rectangle.
            // We use the window's view and an empty rectangle at its center.
            let bounds = unsafe { ns_view.bounds() };
            let centered_rect = NSRect::new(
                (bounds.size.width / 2.0, bounds.size.height / 2.0).into(),
                (0.0, 0.0).into(),
            );

            unsafe {
                // The `NSSharingServicePicker` presents as a popover anchored to the view.
                picker.showRelativeToRect_ofView_preferredEdge(
                    centered_rect,
                    &ns_view,
                    objc2_app_kit::NSRectEdge::NSMinY,
                );
            }
        });
        Ok(())
    })??; // The first '?' handles errors from the closure, the second from run_on_main_thread itself.

    Ok(())
}

/// Retrieves the native `NSView` pointer from the Tauri window.
fn get_ns_view<R: Runtime>(window: &Window<R>) -> Result<Id<NSView>, Error> {
    let handle = window.window_handle()?.raw_window_handle()?;
    if let RawWindowHandle::AppKit(handle) = handle {
        // Security: `raw-window-handle` guarantees this pointer is valid.
        // We retain it to ensure it stays valid while we use it.
        let ns_view = handle.ns_view.as_ptr();
        let ns_view: Id<NSView> = unsafe { Id::retain(ns_view.cast()) }.unwrap();
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