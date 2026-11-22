// NaranjaHub — papelera freedesktop (~/.local/share/Trash).
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// Implementa la especificación "Trash for XDG" de freedesktop.org:
// files/<nombre> + info/<nombre>.trashinfo con la ruta ORIGINAL y la fecha
// de borrado en hora LOCAL. La papelera "home" se busca en $XDG_DATA_HOME
// (por defecto ~/.local/share), igual que hacen GNOME, KDE, COSMIC y XFCE,
// así el archivo aparece de verdad en la papelera del sistema.
// Si el origen está en otro sistema de archivos, hace copia + borrado.

use std::path::{Path, PathBuf};

use super::utils;

#[derive(Clone, Debug)]
pub struct Trashed {
    pub file_path: PathBuf,
    pub info_path: PathBuf,
}

/// Papelera "home" según la especificación: $XDG_DATA_HOME/Trash o
/// ~/.local/share/Trash. `data_home` se pasa aparte para poder simular.
pub fn trash_root_for(data_home: Option<&str>, home: &Path) -> PathBuf {
    match data_home.map(|d| d.trim()).filter(|d| !d.is_empty()) {
        Some(dir) => PathBuf::from(dir).join("Trash"),
        None => home.join(".local/share/Trash"),
    }
}

pub fn default_trash_root(home: &Path) -> PathBuf {
    trash_root_for(std::env::var("XDG_DATA_HOME").ok().as_deref(), home)
}

/// Codificación de la ruta para el campo Path= del .trashinfo.
pub fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        let ch = byte as char;
        if byte.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '~') {
            out.push(ch);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Fecha de borrado en hora LOCAL (la especificación no usa UTC ni sufijo Z).
fn deletion_date() -> String {
    let (y, m, d, h, mi, s) = utils::local_ymd_hms(utils::now_secs());
    format!("{y:04}{m:02}{d:02}T{h:02}{mi:02}{s:02}")
}

fn unique_trash_name(files_dir: &Path, name: &str) -> String {
    let mut candidate = name.to_string();
    let mut counter = 2;
    while files_dir.join(&candidate).exists() {
        candidate = format!("{name}.{counter}");
        counter += 1;
    }
    candidate
}

/// Mueve un archivo a la papelera y registra su .trashinfo.
pub fn trash_file(src: &Path, trash_root: &Path) -> Result<Trashed, String> {
    if !src.exists() {
        return Err(format!("el archivo ya no existe: {}", src.display()));
    }
    let files_dir = trash_root.join("files");
    let info_dir = trash_root.join("info");
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("no se pudo crear la papelera: {e}"))?;
    std::fs::create_dir_all(&info_dir).map_err(|e| format!("no se pudo crear la papelera: {e}"))?;
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("ruta inválida")?;
    let unique = unique_trash_name(&files_dir, &name);
    let target = files_dir.join(&unique);
    if std::fs::rename(src, &target).is_err() {
        // Distinto sistema de archivos: copiar y borrar el original.
        std::fs::copy(src, &target).map_err(|e| format!("no se pudo mover a la papelera: {e}"))?;
        std::fs::remove_file(src).map_err(|e| format!("no se pudo quitar el original: {e}"))?;
    }
    let info_text = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        percent_encode(&src.display().to_string()),
        deletion_date()
    );
    let info_path = info_dir.join(format!("{unique}.trashinfo"));
    std::fs::write(&info_path, info_text)
        .map_err(|e| format!("no se pudo registrar en la papelera: {e}"))?;
    Ok(Trashed {
        file_path: target,
        info_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codificacion_de_rutas() {
        assert_eq!(percent_encode("/home/usuario/Apps"), "/home/usuario/Apps");
        assert_eq!(percent_encode("/a b/c&d"), "/a%20b/c%26d");
    }

    #[test]
    fn raiz_de_papelera_segun_xdg() {
        let home = Path::new("/home/usuario");
        assert_eq!(
            trash_root_for(None, home),
            PathBuf::from("/home/usuario/.local/share/Trash")
        );
        assert_eq!(
            trash_root_for(Some(""), home),
            PathBuf::from("/home/usuario/.local/share/Trash")
        );
        assert_eq!(
            trash_root_for(Some(" /datos/personales "), home),
            PathBuf::from("/datos/personales/Trash")
        );
    }

    #[test]
    fn mover_a_papelera() {
        let base = std::env::temp_dir().join(format!("nh-test-trash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let trash_root = base.join("Trash");
        let src = base.join("Demo.AppImage");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(&src, "datos").unwrap();
        let result = trash_file(&src, &trash_root).expect("debe mover");
        assert!(result.file_path.exists());
        assert!(!src.exists());
        let info = std::fs::read_to_string(&result.info_path).unwrap();
        assert!(info.starts_with("[Trash Info]"));
        assert!(info.contains("Path="));
        assert!(info.contains("DeletionDate="));
        // Colisión de nombre → nombre único
        std::fs::write(&src, "otros datos").unwrap();
        let second = trash_file(&src, &trash_root).expect("segunda vez");
        assert_ne!(second.file_path, result.file_path);
        let _ = std::fs::remove_dir_all(&base);
    }
}
