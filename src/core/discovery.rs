// NaranjaHub — descubrimiento de AppImages y de integraciones previas.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::updater::GhUpdateInfo;
use super::utils;

#[derive(Clone, Debug)]
pub struct AppImageEntry {
    pub path: PathBuf,
    pub name: String,
    pub original_name: Option<String>,
    pub version: Option<String>,
    pub size: u64,
    pub mtime: u64,
    pub executable: bool,
    pub desktop_path: Option<PathBuf>,
    pub update_info: Option<GhUpdateInfo>,
}

pub fn is_appimage_name(file_name: &str) -> bool {
    file_name.to_lowercase().ends_with(".appimage")
}

/// Nombre visible: "MiApp-1.2-x86_64.AppImage" → "MiApp-1.2".
pub fn display_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string());
    utils::clean_display_name(&stem)
}

/// Escanea la carpeta gestionada (nivel superior) buscando .AppImage.
pub fn scan_apps_dir(dir: &Path) -> Vec<AppImageEntry> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if !is_appimage_name(&name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = true;
        let name_visible = display_name(&path);
        found.push(AppImageEntry {
            path,
            name: name_visible,
            original_name: None,
            version: None,
            size: meta.len(),
            mtime,
            executable,
            desktop_path: None,
            update_info: None,
        });
    }
    found
}

/// Lee los accesos directos generados por NaranjaHub y devuelve
/// (ruta AppImage → ruta .desktop) y (ruta AppImage → info de actualización).
pub fn scan_our_desktop_entries(
    desktops_dir: &Path,
) -> (HashMap<PathBuf, PathBuf>, HashMap<PathBuf, GhUpdateInfo>) {
    let mut by_path = HashMap::new();
    let mut by_info = HashMap::new();
    let Ok(entries) = std::fs::read_dir(desktops_dir) else {
        return (by_path, by_info);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if !file_name.starts_with("naranjahub-") || !file_name.ends_with(".desktop") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let keys = super::integration::parse_desktop_file(&text);
        let Some(source) = keys.get("X-NaranjaHub-Source") else {
            continue;
        };
        let source_path = PathBuf::from(source);
        by_path.insert(source_path.clone(), path);
        let raw = keys
            .get("X-NaranjaHub-UpdateInformation")
            .or_else(|| keys.get("X-AppImage-UpdateInformation"));
        if let Some(raw) = raw {
            if let Some(info) = super::updater::parse_update_information(raw) {
                by_info.insert(source_path, info);
            }
        }
    }
    (by_path, by_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deteccion_de_nombres() {
        assert!(is_appimage_name("MiApp.AppImage"));
        assert!(is_appimage_name("MiApp.appimage"));
        assert!(!is_appimage_name("MiApp.png"));
        assert!(!is_appimage_name("MiApp.AppImage.txt"));
    }

    #[test]
    fn nombre_visible() {
        assert_eq!(
            display_name(Path::new("/opt/MiApp-1.2-x86_64.AppImage")),
            "MiApp-1.2"
        );
        assert_eq!(display_name(Path::new("/opt/MiApp.AppImage")), "MiApp");
    }
}
