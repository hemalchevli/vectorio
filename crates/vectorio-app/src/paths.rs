//! Carpeta de datos del usuario (`~/Library/Application Support/Vectorio` en macOS, `%APPDATA%\Vectorio` en Windows).

use std::path::PathBuf;

pub fn support_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Vectorio"))
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            Some(PathBuf::from(appdata).join("Vectorio"))
        } else if let Some(userprofile) = std::env::var_os("USERPROFILE") {
            Some(PathBuf::from(userprofile).join("AppData").join("Roaming").join("Vectorio"))
        } else {
            None
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            Some(PathBuf::from(xdg).join("vectorio"))
        } else if let Some(home) = std::env::var_os("HOME") {
            Some(PathBuf::from(home).join(".local/share/vectorio"))
        } else {
            None
        }
    }
}
