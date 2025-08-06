use crate::models::{ShareDataOptions, ShareFileOptions, ShareTextOptions};
use crate::Error;
use base64::{engine::general_purpose, Engine as _};
use objc2::{
    rc::{autoreleasepool, Retained},
    runtime::AnyObject,
    AnyThread,
};
use objc2_app_kit::{NSSharingServicePicker, NSView};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSString, NSURL};
use raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle};
use std::{io::Write, path::Path, sync::mpsc};
use tauri::{Runtime, Window};
use tempfile::{Builder, NamedTempFile};

pub fn share_text<R: Runtime>(window: Window<R>, options: ShareTextOptions) -> Result<(), Error> {
    show_share_sheet_with_text(window, options.text)
}

pub fn share_data<R: Runtime>(window: Window<R>, options: ShareDataOptions) -> Result<(), Error> {
    let temp_file = create_temp_file_for_data(&options)?;
    let path_str = temp_file.path().to_string_lossy().to_string();
    show_share_sheet_with_file(window, path_str)
}

pub fn share_file<R: Runtime>(window: Window<R>, options: ShareFileOptions) -> Result<(), Error> {
    // Security: Ensure the file exists.
    if !Path::new(&options.path).exists() {
        return Err(Error::InvalidArgs(format!(
            "File does not exist at path: {}",
            options.path
        )));
    }
    show_share_sheet_with_file(window, options.path)
}

pub fn cleanup() -> Result<(), Error> {
    // Temporary file management on macOS is automatic thanks to `NamedTempFile`.
    // This function can remain empty or perform a more thorough cleanup if needed.
    Ok(())
}

/// Displays the native macOS share sheet with text content.
fn show_share_sheet_with_text<R: Runtime>(window: Window<R>, text: String) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel();
    let window_clone = window.clone();

    window.run_on_main_thread(move || {
        let result = (|| -> Result<(), Error> {
            let ns_view = get_ns_view(&window_clone)?;
            autoreleasepool(|_pool| {
                let text_obj = NSString::from_str(&text);
                let objects = vec![&*text_obj as &AnyObject];
                let items = NSArray::from_slice(&objects);

                let picker = unsafe {
                    NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &*items)
                };
                let bounds = ns_view.bounds();
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
        tx.send(result)
            .expect("Failed to send result from main thread");
    })?;

    rx.recv()??;
    Ok(())
}

/// Displays the native macOS share sheet with file content.
fn show_share_sheet_with_file<R: Runtime>(
    window: Window<R>,
    file_path: String,
) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel();
    let window_clone = window.clone();

    window.run_on_main_thread(move || {
        let result = (|| -> Result<(), Error> {
            let ns_view = get_ns_view(&window_clone)?;
            autoreleasepool(|_pool| {
                let url = unsafe { NSURL::fileURLWithPath(&NSString::from_str(&file_path)) };
                let objects = vec![&*url as &AnyObject];
                let items = NSArray::from_slice(&objects);

                let picker = unsafe {
                    NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &*items)
                };
                let bounds = ns_view.bounds();
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
        tx.send(result)
            .expect("Failed to send result from main thread");
    })?;

    rx.recv()??;
    Ok(())
}

/// Retrieves the native `NSView` pointer from the Tauri window, compatible with `raw-window-handle`.
fn get_ns_view<R: Runtime>(window: &Window<R>) -> Result<Retained<NSView>, Error> {
    let window_handle: WindowHandle<'_> = window.window_handle()?;
    if let RawWindowHandle::AppKit(handle) = window_handle.as_raw() {
        let ns_view_ptr = handle.ns_view.as_ptr();
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
        .ok_or_else(|| {
            Error::InvalidArgs("File name contains invalid UTF-8 characters.".to_string())
        })?;
    let temp_dir = std::env::temp_dir();
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
