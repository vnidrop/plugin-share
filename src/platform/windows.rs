use crate::models::{ShareDataOptions, ShareFileOptions, ShareTextOptions};
use base64::{engine::general_purpose, Engine as _};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::ApplicationModel::DataTransfer::DataTransferManager;
use windows::Storage::IStorageItem;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tauri::{Runtime, Window};
use tempfile::{Builder, NamedTempFile};
use windows::{
    core::{HSTRING},
    Foundation::{TypedEventHandler},
    Storage::StorageFile,
    Win32::{
        UI::Shell::IDataTransferManagerInterop,
        Foundation::HWND,
        System::{
            WinRT::{
                RoInitialize,
                RO_INIT_SINGLETHREADED,
            },
        },
    },
};
use windows_core::Interface;
use crate::Error;

// A helper to map the detailed windows::core::Error into our plugin's simpler error type.
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::NativeApi(err.message().to_string())
    }
}

pub fn share_text<R: Runtime>(
    window: Window<R>,
    options: ShareTextOptions,
) -> Result<(), Error> {
    let share_payload = SharePayload::Text(options);
    show_share_sheet(window, share_payload)
}

pub fn share_data<R: Runtime>(
    window: Window<R>,
    options: ShareDataOptions,
) -> Result<(), Error> {
    let temp_file = create_temp_file_for_data(&options)?;
    let file_path = temp_file
       .path()
       .to_str()
       .ok_or_else(|| Error::TempFile("Invalid temporary file path".to_string()))?
       .to_string();

    let file_options = ShareFileOptions {
        path: file_path,
        title: options.title,
    };

    // The temp_file will be automatically deleted when it goes out of scope
    // after the share sheet is closed.
    show_share_sheet(window, SharePayload::Files(vec![file_options.path]))
}

pub fn share_file<R: Runtime>(
    window: Window<R>,
    options: ShareFileOptions,
) -> Result<(), Error> {
    // We don't check for file existence to mitigate TOCTOU
    show_share_sheet(window, SharePayload::Files(vec![options.path]))
}

pub fn cleanup() -> Result<(), Error> {
    let temp_dir = get_plugin_temp_dir()?;
    if temp_dir.exists() {
        std::fs::remove_dir_all(temp_dir)
           .map_err(|e| Error::TempFile(format!("Failed to cleanup temp dir: {}", e)))?;
    }
    Ok(())
}

enum SharePayload {
    Text(ShareTextOptions),
    Files(Vec<String>),
}

/// The main entry point that handles showing the share sheet.
/// It ensures all UI operations are run on the main thread.
fn show_share_sheet<R: Runtime>(window: Window<R>, payload: SharePayload) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel();
    
    window
        .run_on_main_thread(move || {
            let result = (|| -> Result<(), Error> {
                initialize_winrt_thread()?;
                let hwnd = get_hwnd(&window)?;
                let dtm = get_data_transfer_manager(hwnd)?;

                let data_requested_handler =
                    TypedEventHandler::new(move |_, args: windows::core::Ref<'_, windows::ApplicationModel::DataTransfer::DataRequestedEventArgs>| -> windows::core::Result<()> {
                        if let Some(request_args) = (*args).as_ref() {
                            let request = request_args.Request()?;
                            let data = request.Data()?;
                            let properties = data.Properties()?;

                            match &payload {
                                SharePayload::Text(options) => {
                                    if let Some(title) = &options.title {
                                        properties.SetTitle(&HSTRING::from(title))?;
                                    }
                                    data.SetText(&HSTRING::from(&options.text))?;
                                }
                                SharePayload::Files(paths) => {
                                    let deferral = request.GetDeferral()?;
                                    let paths_clone = paths.clone();
                                    let data_clone = data.clone();

                                    tauri::async_runtime::spawn(async move {
                                        let mut storage_items: Vec<windows::Storage::IStorageItem> = Vec::new();
                                        let mut first_error: Option<windows::core::Error> = None;

                                        for path in paths_clone {
                                            let op_result = StorageFile::GetFileFromPathAsync(&HSTRING::from(path));
                                            if let Err(e) = op_result {
                                                log::error!("Failed to create file operation: {}", e);
                                                if first_error.is_none() { first_error = Some(e); }
                                                continue;
                                            }
                                            
                                            match op_result.unwrap().get() {
                                                Ok(file) => {
                                                    if let Ok(item) = file.cast() {
                                                        storage_items.push(item);
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Failed to get file: {}", e);
                                                    if first_error.is_none() { first_error = Some(e); }
                                                }
                                            }
                                        }
                                        
                                        if !storage_items.is_empty() {
                                            let _ = data_clone.SetStorageItemsReadOnly(storage_items)?;
                                        }
                                        // Signal that we are done with the async operation.
                                        deferral.Complete()?;
                                        Ok::<(), windows::core::Error>(())
                                    });
                                }
                            }
                        }
                        Ok(())
                    });

                let token = dtm.DataRequested(&data_requested_handler)?;
                DataTransferManager::ShowShareUI()?;
                dtm.RemoveDataRequested(token)?;

                Ok(())
            })();
            
            tx.send(result).ok();
        })
        .map_err(|e| Error::NativeApi(e.to_string()))?;

    rx.recv()
        .map_err(|_| Error::NativeApi("Failed to receive result from main thread".to_string()))?
}

/// Initializes the Windows Runtime on the current thread.
fn initialize_winrt_thread() -> Result<(), Error> {
    // RoInitialize can be called multiple times on the same thread.
    // It will return S_FALSE if already initialized, which is not an error.
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }
       .map_err(|e| Error::NativeApi(format!("Failed to initialize WinRT: {}", e)))
}

/// Retrieves the native window handle (HWND) from the Tauri window.
fn get_hwnd<R: Runtime>(window: &Window<R>) -> Result<HWND, Error> {
    let handle = window
        .window_handle()
        .map_err(|e| Error::NativeApi(e.to_string()))?;

    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut std::ffi::c_void)),
        _ => Err(Error::NativeApi(
            "Unsupported window handle type".to_string(),
        )),
    }
}

/// Gets an instance of the DataTransferManager associated with the window's HWND.
/// This is the required method for desktop (non-UWP) applications. [1]
fn get_data_transfer_manager(hwnd: HWND) -> Result<DataTransferManager, Error> {
    let interop = 
        windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>()?;
    let dtm = unsafe { interop.GetForWindow(hwnd) }?;
    Ok(dtm)
}

/// Returns the path to a dedicated, secure directory for this plugin's temporary files.
fn get_plugin_temp_dir() -> Result<PathBuf, Error> {
    let dir = std::env::temp_dir().join("tauri-plugin-share");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
           .map_err(|e| Error::TempFile(format!("Failed to create temp dir: {}", e)))?;
    }
    Ok(dir)
}

/// Creates a secure temporary file from Base64 data.
fn create_temp_file_for_data(options: &ShareDataOptions) -> Result<NamedTempFile, Error> {
    let decoded_bytes = general_purpose::STANDARD
       .decode(&options.data)
       .map_err(|_| Error::InvalidArgs("Invalid Base64 data provided".to_string()))?;

    // Security: Sanitize the filename to prevent path traversal attacks.
    // We only use the filename part and ignore any directory structure.
    let sanitized_name = Path::new(&options.name)
       .file_name()
       .ok_or_else(|| Error::InvalidArgs("Invalid file name provided".to_string()))?
       .to_str()
       .ok_or_else(|| Error::InvalidArgs("File name contains invalid UTF-8".to_string()))?;

    let temp_dir = get_plugin_temp_dir()?;

    // Use the tempfile crate's builder for secure, unique file creation.
    let mut temp_file = Builder::new()
       .prefix(&format!("{}-", uuid::Uuid::new_v4())) // Guarantees uniqueness
       .suffix(&format!("-{}", sanitized_name))
       .tempfile_in(temp_dir)
       .map_err(|e| Error::TempFile(format!("Failed to create temp file: {}", e)))?;

    temp_file
       .write_all(&decoded_bytes)
       .map_err(|e| Error::TempFile(format!("Failed to write to temp file: {}", e)))?;

    Ok(temp_file)
}