use base64::{engine::general_purpose, Engine as _};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::ApplicationModel::DataTransfer::{DataTransferManager, DataRequestedEventArgs};
use windows::Storage::IStorageItem;
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tauri::{Runtime, Window};
use tempfile::{Builder, NamedTempFile};
use windows::{
    core::{HSTRING, Interface},
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
use windows_collections::IIterable;
use crate::{Error, SharedFile, ShareOptions, CanShareResult};

// This thread-local static variable is the key to solving the lifetime issue.
// It will hold the DataTransferManager and its event registration token, keeping them
// alive for the duration of the asynchronous share operation. It's only accessible
// on the main thread, which is safe for these non-thread-safe WinRT types.
thread_local! {
    static SHARE_STATE: RefCell<Option<(DataTransferManager, i64)>> = RefCell::new(None);
}

// A helper to map the detailed windows::core::Error into our plugin's simpler error type.
impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::NativeApi(err.message().to_string())
    }
}

pub fn cleanup() -> Result<(), Error> {
    let temp_dir = get_plugin_temp_dir()?;
    if temp_dir.exists() {
        std::fs::remove_dir_all(temp_dir)
           .map_err(|e| Error::TempFile(format!("Failed to cleanup temp dir: {}", e)))?;
    }
    Ok(())
}

pub fn can_share() -> Result<CanShareResult, Error> {
    Ok(CanShareResult { value: true })
}


pub fn share<R: Runtime>(window: Window<R>, options: ShareOptions) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel();
    let win_clone = window.clone();
    
    window.run_on_main_thread(move || {
        let options_arc = std::sync::Arc::new(options.clone());
        let result = (|| -> Result<(), Error> {
            initialize_winrt_thread()?;
            let hwnd = get_hwnd(&win_clone)?;
            let (dtm, interop) = get_data_transfer_manager(hwnd)?;

            let data_requested_handler = TypedEventHandler::new({
                let options_clone = options_arc.clone();
                move |_, args: windows::core::Ref<'_, DataRequestedEventArgs>| -> windows::core::Result<()> {
                    if let Some(request_args) = (*args).as_ref() {
                        let request = request_args.Request()?;
                        let data = request.Data()?;
                        let properties = data.Properties()?;

                        if let Some(title) = &options_clone.title {
                            properties.SetTitle(&HSTRING::from(title))?;
                        }

                        let combined_text = match (&options_clone.text, &options_clone.url) {
                            (Some(t), Some(u)) => format!("{}\n{}", t, u),
                            (Some(t), None) => t.clone(),
                            (None, Some(u)) => u.clone(),
                            (None, None) => String::new(),
                        };

                        if !combined_text.is_empty() {
                            data.SetText(&HSTRING::from(combined_text))?;
                        }

                        if let Some(files) = &options_clone.files {
                            let deferral = request.GetDeferral()?;
                            let data_clone = data.clone();

                            tauri::async_runtime::spawn({
                                let files = files.clone(); // clone to move into async
                                async move {
                                    let mut storage_items: Vec<IStorageItem> = Vec::new();

                                    let mut temp_files: Vec<NamedTempFile> = Vec::new();

                                    for file in files {
                                        match create_temp_file_for_data(&file) {
                                            Ok(temp_file) => {
                                                let temp_path = temp_file.into_temp_path();
                                                let path_str = temp_path.to_string_lossy().to_string();

                                                match StorageFile::GetFileFromPathAsync(&HSTRING::from(path_str)) {
                                                    Ok(op) => match op.get() {
                                                        Ok(storage_file) => {
                                                            if let Ok(item) = storage_file.cast() {
                                                                storage_items.push(item);
                                                            }
                                                        }, 
                                                        Err(e) => eprintln!("Failed to get storage file: {}", e),
                                                    }, 
                                                    Err(e) => eprintln!("Failed to get file from path: {}", e),
                                                }
                                            },
                                            Err(e) => eprintln!("Failed to create temp file: {}", e),
                                        }
                                    }

                                    if !storage_items.is_empty() {
                                        let options_items = storage_items.into_iter().map(Some).collect::<Vec<_>>();
                                        let iterable_items: Result<IIterable<IStorageItem>, _> = options_items.try_into();
                                        
                                        match iterable_items {
                                            Ok(items) => {
                                                if let Err(e) = data_clone.SetStorageItemsReadOnly(&items) {
                                                    println!("Failed to set storage items on data package: {}", e);
                                                }
                                            },
                                            Err(e) => {
                                                println!("Failed to convert Vec to IIterable: {}", e);
                                            }
                                        }
                                    }
                                    deferral.Complete()?;
                                    Ok::<(), windows::core::Error>(())
                                }

                            });
                        }

                        SHARE_STATE.with(|state| {
                            if let Some((manager, token)) = state.borrow_mut().take() {
                                let _ = manager.RemoveDataRequested(token);
                            }
                        });
                    }
                    Ok(())
                }
            });

            let token = dtm.DataRequested(&data_requested_handler)?;
            
            SHARE_STATE.with(|state| {
                *state.borrow_mut() = Some((dtm, token));
            });

            unsafe { interop.ShowShareUIForWindow(hwnd) }?;
            Ok(())
        })();
        tx.send(result).ok();
    })?;

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
fn get_data_transfer_manager(hwnd: HWND) -> Result<(DataTransferManager, IDataTransferManagerInterop), Error> {
    let interop = 
        windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>()?;
    let dtm = unsafe { interop.GetForWindow(hwnd) }?;
    Ok((dtm, interop))
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
fn create_temp_file_for_data(file: &SharedFile) -> Result<NamedTempFile, Error> {
    let decoded_bytes = general_purpose::STANDARD
       .decode(&file.data)
       .map_err(|_| Error::InvalidArgs("Invalid Base64 data provided".to_string()))?;

    // Security: Sanitize the filename to prevent path traversal attacks.
    // We only use the filename part and ignore any directory structure.
    let sanitized_name = Path::new(&file.name)
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