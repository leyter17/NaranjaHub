// NaranjaHub — almacén persistente de direcciones de actualización manuales.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// Guarda la información de actualización definida a mano por el usuario
// (ruta de la AppImage → cadena gh-releases-zsync) para que sobreviva al
// reinicio, esté o no la AppImage integrada en el menú de aplicaciones.
// Se escribe de forma atómica (archivo temporal + rename) y con claves
// ordenadas para que el JSON resultante sea estable y legible.

use std::collections::BTreeMap;
use std::path::Path;

/// Ruta de la AppImage → cadena gh-releases-zsync escrita a mano.
pub type UpdateOverrides = BTreeMap<String, String>;

/// Carga el almacén. Un archivo ausente o corrupto devuelve un almacén
/// vacío en lugar de romper el arranque.
pub fn load(path: &Path) -> UpdateOverrides {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Guarda el almacén completo de forma atómica.
pub fn save(path: &Path, overrides: &UpdateOverrides) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("no se pudo crear {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(overrides).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)
        .map_err(|e| format!("no se pudo escribir {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("no se pudo guardar {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardar_y_cargar() {
        let base = std::env::temp_dir().join(format!("nh-test-updstore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::create_dir_all(&base);
        let path = base.join("updates.json");
        let mut map = UpdateOverrides::new();
        map.insert(
            "/apps/Demo.AppImage".to_string(),
            "gh-releases-zsync|u|r|latest|Demo-*.AppImage".to_string(),
        );
        map.insert(
            "/apps/Otra.AppImage".to_string(),
            "gh-releases-zsync|u2|r2|latest|*.AppImage".to_string(),
        );
        save(&path, &map).expect("guardar");
        let loaded = load(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(
            loaded.get("/apps/Demo.AppImage").map(String::as_str),
            Some("gh-releases-zsync|u|r|latest|Demo-*.AppImage")
        );
        // Las claves se guardan ordenadas (JSON estable).
        let text = std::fs::read_to_string(&path).unwrap();
        let demo_pos = text.find("/apps/Demo.AppImage").unwrap();
        let otra_pos = text.find("/apps/Otra.AppImage").unwrap();
        assert!(demo_pos < otra_pos, "las claves deben salir ordenadas");
        // Archivo corrupto → almacén vacío, sin romper el arranque.
        std::fs::write(&path, "{no es json").unwrap();
        assert!(load(&path).is_empty());
        // Archivo que no existe → vacío.
        assert!(load(&base.join("inexistente.json")).is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }
}
