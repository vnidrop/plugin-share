use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Manages temporary files created by the plugin.
pub struct PluginTempFileManager {
    /// A list of all managed temporary files.
    pub managed_files: Arc<Mutex<Vec<PathBuf>>>,
}

impl PluginTempFileManager {
    pub fn new() -> Self {
        Self {
            managed_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_file(&self, path: PathBuf) -> Result<(), String> {
        let mut files = self
            .managed_files
            .lock()
            .map_err(|e| format!("Failed to lock mutex: {}", e))?;
        files.push(path);
        Ok(())
    }

    pub fn remove_and_delete_file(&self, path_to_remove: &PathBuf) -> Result<(), String> {
        let mut files = self
            .managed_files
            .lock()
            .map_err(|e| format!("Failed to lock mutex: {}", e))?;
        if let Some(index) = files.iter().position(|p| p == path_to_remove) {
            let file_path = files.remove(index);
            std::fs::remove_file(&file_path) // [4]
               .map_err(|e| format!("Failed to delete file {}: {}", file_path.display(), e))?;
            Ok(())
        } else {
            Err(format!(
                "File not found in managed list: {}",
                path_to_remove.display()
            ))
        }
    }

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
