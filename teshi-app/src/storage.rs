//! Simple key-value storage using the browser's localStorage API.
//!
//! Used to persist Gherkin feature files and user settings across sessions.
//! For larger files or binary data, this can be upgraded to use the
//! Origin Private File System (OPFS).

use web_sys::{window, Storage};

const STORAGE_PREFIX: &str = "teshi:";

fn storage() -> Option<Storage> {
    window()?.local_storage().ok()?
}

/// Save a file (or any text value) to localStorage under a namespaced key.
pub fn save_file(name: &str, content: &str) {
    if let Some(storage) = storage() {
        let key = format!("{}{}", STORAGE_PREFIX, name);
        if let Err(e) = storage.set_item(&key, content) {
            log::error!("Failed to save '{}': {:?}", name, e);
        }
    }
}

/// Load a file (or any text value) from localStorage.
pub fn load_file(name: &str) -> Option<String> {
    let storage = storage()?;
    let key = format!("{}{}", STORAGE_PREFIX, name);
    storage.get_item(&key).ok()?
}

/// Delete a saved file from localStorage.
pub fn delete_file(name: &str) {
    if let Some(storage) = storage() {
        let key = format!("{}{}", STORAGE_PREFIX, name);
        if let Err(e) = storage.remove_item(&key) {
            log::error!("Failed to delete '{}': {:?}", name, e);
        }
    }
}

/// List all stored file names (strips the prefix).
pub fn list_files() -> Vec<String> {
    let mut files = Vec::new();
    let storage = match storage() {
        Some(s) => s,
        None => return files,
    };
    let len = match storage.length() {
        Ok(l) => l,
        Err(_) => return files,
    };
    for i in 0..len {
        if let Ok(Some(key)) = storage.key(i) {
            if let Some(name) = key.strip_prefix(STORAGE_PREFIX) {
                files.push(name.to_string());
            }
        }
    }
    files
}

/// Save a feature file content. The name should be a relative path like "features/login.feature".
pub fn save_feature(name: &str, content: &str) {
    save_file(&format!("feature:{}", name), content);
}

/// Load a feature file content.
pub fn load_feature(name: &str) -> Option<String> {
    load_file(&format!("feature:{}", name))
}

/// Delete a feature file.
#[allow(dead_code)]
pub fn delete_feature(name: &str) {
    delete_file(&format!("feature:{}", name));
}

/// List all stored feature file names.
pub fn list_features() -> Vec<String> {
    list_files()
        .into_iter()
        .filter(|k| k.starts_with("feature:"))
        .map(|k| k["feature:".len()..].to_string())
        .collect()
}

/// Save user settings (API key, Runner URL, etc.) as JSON.
#[allow(dead_code)]
pub fn save_settings(settings: &str) {
    save_file("settings", settings);
}

/// Load user settings as JSON string.
pub fn load_settings() -> Option<String> {
    load_file("settings")
}
