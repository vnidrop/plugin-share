use crate::models::{Error, ShareDataOptions, ShareFileOptions, ShareTextOptions};
use base64::{engine::general_purpose, Engine as _};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::ApplicationModel::DataTransfer::DataTransferManager;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{Runtime, Window};
use tempfile::{Builder, NamedTempFile};
use windows::{
    core::{HSTRING, IInspectable},
    Foundation::{ TypedEventHandler},
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
    // Security: Ensure the file exists before attempting to share.
    if!Path::new(&options.path).exists() {
        return Err(Error::InvalidArgs(format!(
            "File does not exist at path: {}",
            options.path
        )));
    }
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
    window
       .run_on_main_thread(move |

| {
            // We must initialize the WinRT Core on the thread that will show the UI.
            // This is a requirement for DataTransferManager.
            initialize_winrt_thread()?;

            let hwnd = get_hwnd(&window)?;
            let dtm = get_data_transfer_manager(hwnd)?;

            let data_requested_handler =
                TypedEventHandler::new(move |_, args| -> windows::core::Result<()> {
                    if let Some(request) = args {
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
                                // The handler for files must be async. We need to get a deferral
                                // to prevent the main thread from moving on before we're done.
                                let deferral = request.GetDeferral()?;
                                let paths_clone = paths.clone();

                                // Spawn a future to handle the async file loading.
                                tauri::async_runtime::spawn(async move {
                                    let mut storage_items: Vec<IInspectable> = Vec::new();
                                    for path in paths_clone {
                                        match StorageFile::GetFileFromPathAsync(&HSTRING::from(path)) {
                                            Ok(op) => {
                                                if op.await.is_ok() {
                                                    if let Ok(file) = op.GetResults() {
                                                        storage_items.push(file.into());
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log::error!("Failed to get StorageFile for path {}: {}", path, e);
                                            }
                                        }
                                    }

                                    if!storage_items.is_empty() {
                                        // This is safe because we are in an async context
                                        // that was spawned from the main thread handler.
                                        let _ = data.SetStorageItems(&storage_items);
                                    }
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
        })
       .map_err(|e| Error::NativeApi(e.to_string()))
}

/// Helper function to convert a Vec of file paths to a Vec of WinRT StorageFile objects.
async fn get_storage_files_from_paths(paths: &[String]) -> Result<Vec<StorageFile>, Error> {
    let mut storage_files = Vec::new();
    for path in paths {
        let h_path = HSTRING::from(path.as_str());
        // GetFileFromPathAsync returns an IAsyncOperation that we can .await.
        let op = StorageFile::GetFileFromPathAsync(&h_path)?;
        match op.await {
            Ok(file) => storage_files.push(file),
            Err(e) => log::error!("Failed to get StorageFile for path {}: {}", path, e),
        }
    }
    Ok(storage_files)
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
    match window.window_handle()?.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as isize)),
        _ => Err(Error::NativeApi(
            "Unsupported window handle type".to_string(),
        )),
    }
}

/// Gets an instance of the DataTransferManager associated with the window's HWND.
/// This is the required method for desktop (non-UWP) applications. [1]
fn get_data_transfer_manager(hwnd: HWND) -> Result<DataTransferManager, Error> {
    let interop: IDataTransferManagerInterop = windows::core::factory::<DataTransferManager, _>()?;
    let dtm: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };
    Ok(dtm)
}

/// Returns the path to a dedicated, secure directory for this plugin's temporary files.
fn get_plugin_temp_dir() -> Result<PathBuf, Error> {
    let dir = std::env::temp_dir().join("tauri-plugin-share");
    if!dir.exists() {
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