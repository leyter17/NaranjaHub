// NaranjaHub — configuración persistente en ~/.config/NaranjaHub/config.json.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::i18n::Lang;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub lang: Lang,
    #[serde(default = "default_apps_dir")]
    pub apps_dir: String,
    #[serde(default)]
    pub check_updates_on_start: bool,
    #[serde(default = "default_true")]
    pub confirm_deletes: bool,
    /// Backend gráfico: "auto" (X11 primero, luego Wayland), "x11" o "wayland".
    /// X11/XWayland es el único con arrastrar y soltar funcional de momento.
    #[serde(default)]
    pub backend: String,
}

fn default_true() -> bool {
    true
}

fn default_apps_dir() -> String {
    dirs::home_dir()
        .map(|h| h.join("Applications").display().to_string())
        .unwrap_or_else(|| "~/Applications".to_string())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lang: Lang::Es,
            apps_dir: default_apps_dir(),
            check_updates_on_start: false,
            confirm_deletes: true,
            backend: String::new(),
        }
    }
}

/// Backend elegido: "auto", "x11" o "wayland". Ignora valores desconocidos.
pub fn normalize_backend(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "x11" => "x11".to_string(),
        "wayland" => "wayland".to_string(),
        _ => "auto".to_string(),
    }
}

impl Config {
    pub fn backend_pref(&self) -> String {
        normalize_backend(&self.backend)
    }
}

pub fn load(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("no se pudo crear {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)
        .map_err(|e| format!("no se pudo escribir {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("no se pudo guardar la configuración: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn valores_por_defecto() {
        let cfg = Config::default();
        assert_eq!(cfg.lang, Lang::Es);
        assert!(cfg.confirm_deletes);
        assert!(!cfg.check_updates_on_start);
        assert!(cfg.apps_dir.ends_with("Applications"));
        assert_eq!(cfg.backend_pref(), "auto");
    }

    #[test]
    fn backend_normalizado() {
        assert_eq!(normalize_backend("x11"), "x11");
        assert_eq!(normalize_backend(" Wayland "), "wayland");
        assert_eq!(normalize_backend("otro"), "auto");
        assert_eq!(normalize_backend(""), "auto");
    }

    #[test]
    fn guardar_y_cargar() {
        let base = std::env::temp_dir().join(format!("nh-test-cfg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let path: PathBuf = base.join("config.json");
        let cfg = Config {
            lang: Lang::En,
            apps_dir: "/tmp/apps".to_string(),
            check_updates_on_start: true,
            ..Default::default()
        };
        save(&path, &cfg).expect("guardar");
        let loaded = load(&path);
        assert_eq!(loaded, cfg);
        // Archivo corrupto: vuelve a los valores por defecto
        std::fs::write(&path, "{esto no es json").unwrap();
        assert_eq!(load(&path), Config::default());
        let _ = std::fs::remove_dir_all(&base);
    }
}
