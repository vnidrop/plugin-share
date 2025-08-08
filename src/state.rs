use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Manages the lifecycle of temporary files created by the plugin.
///
/// This struct holds a thread-safe list of `PathBuf` for all temporary files
/// that have been created and need to be cleaned up. It's intended to be
/// managed as a Tauri state.
pub struct PluginTempFileManager {
    /// A thread-safe vector to store the paths of temporary files.
    pub managed_files: Arc<Mutex<Vec<PathBuf>>>,
}

impl PluginTempFileManager {
    pub fn new() -> Self {
        Self {
            managed_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Cleans up all files currently managed by this instance.
    ///
    /// This method iterates through the list of file paths, attempts to
    /// delete each file, and clears the list. It also handles a poisoned
    /// mutex gracefully by recovering the inner data and continuing the cleanup.
    pub fn cleanup_all_managed_files(&self) {
        let mut files = match self.managed_files.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Mutex was poisoned during cleanup: {:?}", poisoned);
                poisoned.into_inner()
            }
        };
        let mut errors = Vec::new();
        for path in files.drain(..) {
            if let Err(e) = std::fs::remove_file(&path) {
                errors.push(format!("Failed to delete file {}: {}", path.display(), e));
            }
        }
        if !errors.is_empty() {
            eprintln!("Errors during cleanup: {:?}", errors);
        }
    }
}
