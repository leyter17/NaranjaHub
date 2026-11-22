// NaranjaHub — integración de AppImages: copia, permisos, iconos y
// accesos directos .desktop compatibles con GNOME, KDE, COSMIC y XFCE.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// La extracción de iconos y metadatos usa `--appimage-extract`, por lo que
// NO se necesita FUSE ni ninguna aplicación externa. La información de
// actualización se busca en el .desktop interno y, si no está, en la
// sección ELF `.upd_info` que escribe appimagetool (lo habitual).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::discovery;
use super::utils;

#[derive(Clone, Debug)]
pub struct DirPaths {
    pub apps_dir: PathBuf,
    pub icons_dir: PathBuf,
    /// Tema de iconos del usuario (~/.local/share/icons/hicolor o $XDG_DATA_HOME).
    pub hicolor_dir: PathBuf,
    pub desktops_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct IntegrateOptions {
    pub create_entry: bool,
    pub move_original: bool,
}

impl Default for IntegrateOptions {
    fn default() -> Self {
        Self {
            create_entry: true,
            move_original: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IntegrateOutcome {
    pub new_path: PathBuf,
    pub desktop_path: Option<PathBuf>,
    pub app_name: String,
}

#[derive(Clone, Debug, Default)]
pub struct ExtractedMeta {
    pub name: Option<String>,
    pub version: Option<String>,
    pub comment: Option<String>,
    pub categories: Option<String>,
    pub update_information: Option<String>,
    /// Contenido del icono extraído (se guarda en bytes porque la carpeta
    /// temporal de extracción se borra al terminar).
    pub icon_data: Option<Vec<u8>>,
    /// Extensión detectada por contenido: png, svg, xpm, jpg...
    pub icon_ext: Option<String>,
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn make_temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Copia un archivo informando del progreso (bytes copiados, bytes totales).
pub fn copy_with_progress(
    src: &Path,
    dst: &Path,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<(), String> {
    let total = std::fs::metadata(src).map(|m| m.len()).ok();
    let mut input =
        std::fs::File::open(src).map_err(|e| format!("no se pudo abrir {}: {e}", src.display()))?;
    let mut output = std::fs::File::create(dst)
        .map_err(|e| format!("no se pudo crear {}: {e}", dst.display()))?;
    let mut buffer = [0u8; 262_144];
    let mut copied: u64 = 0;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|e| format!("error al leer: {e}"))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|e| format!("error al escribir: {e}"))?;
        copied += read as u64;
        progress(copied, total);
    }
    output.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Devuelve una ruta destino única: "X.AppImage" → "X (2).AppImage"...
pub fn unique_target(dir: &Path, file_name: &str) -> PathBuf {
    let base = PathBuf::from(file_name);
    let stem = base
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = base
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    let mut candidate = dir.join(&base);
    let mut counter = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem} ({counter}){ext}"));
        counter += 1;
    }
    candidate
}

pub fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("no se pudo dar permiso de ejecución: {e}"))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// Identificador seguro para nombres de archivos (icono, .desktop).
pub fn slug_for(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string());
    utils::sanitize_filename(&stem)
}

fn run_extract(appimage: &Path, work_dir: &Path, pattern: &str) -> Result<(), String> {
    let output = Command::new(appimage)
        .arg("--appimage-extract")
        .arg(pattern)
        .current_dir(work_dir)
        .output()
        .map_err(|e| format!("no se pudo ejecutar --appimage-extract: {e}"))?;
    if !output.status.success() {
        return Err("la AppImage no soporta --appimage-extract (¿tipo 1?)".to_string());
    }
    Ok(())
}

fn symlink_target_rel(work_dir: &Path) -> Option<String> {
    let dir_icon = work_dir.join("squashfs-root").join(".DirIcon");
    let target = std::fs::read_link(dir_icon).ok()?;
    if target.is_absolute() {
        return None;
    }
    Some(target.to_string_lossy().to_string())
}

/// Resuelve el icono extraído (.DirIcon puede ser un PNG o un enlace).
fn resolve_extracted_icon(work_dir: &Path) -> Option<PathBuf> {
    let root = work_dir.join("squashfs-root");
    let dir_icon = root.join(".DirIcon");
    let path = if dir_icon.is_file() {
        Some(dir_icon)
    } else if dir_icon.is_symlink() {
        let target = std::fs::read_link(&dir_icon).ok()?;
        let resolved = if target.is_absolute() {
            target
        } else {
            root.join(target)
        };
        if resolved.is_file() {
            Some(resolved)
        } else {
            None
        }
    } else {
        None
    }?;
    let too_big = path
        .metadata()
        .ok()
        .map(|m| m.len() > 4 * 1024 * 1024)
        .unwrap_or(true);
    if too_big {
        return None;
    }
    Some(path)
}

fn collect_desktops(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_desktops(&path, out, depth + 1);
        } else if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("desktop"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn find_first_application_desktop(work_dir: &Path) -> Option<PathBuf> {
    let root = work_dir.join("squashfs-root");
    let mut candidates = Vec::new();
    collect_desktops(&root, &mut candidates, 0);
    candidates.sort();
    for candidate in candidates {
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            let keys = parse_desktop_file(&text);
            let is_app = keys
                .get("Type")
                .map(|t| t.eq_ignore_ascii_case("application"))
                .unwrap_or(false)
                || keys.contains_key("Exec");
            if is_app {
                return Some(candidate);
            }
        }
    }
    None
}

/// Parsea la sección [Desktop Entry] de un archivo .desktop.
pub fn parse_desktop_file(text: &str) -> HashMap<String, String> {
    let mut keys = HashMap::new();
    let mut in_main = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_main = line.eq_ignore_ascii_case("[desktop entry]");
            continue;
        }
        if !in_main || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            keys.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    keys
}

/// Extrae icono y metadatos de una AppImage (sin FUSE, sin apps externas).
pub fn extract_metadata(appimage: &Path) -> ExtractedMeta {
    let mut meta = ExtractedMeta::default();
    let tmp = make_temp_dir("naranjahub-meta");
    let _ = (|| -> Result<(), String> {
        run_extract(appimage, &tmp, ".DirIcon")?;
        let mut icon = resolve_extracted_icon(&tmp);
        if icon.is_none() {
            // .DirIcon suele ser un enlace simbólico a la ruta real del icono
            if let Some(rel) = symlink_target_rel(&tmp) {
                let _ = run_extract(appimage, &tmp, &rel);
                icon = resolve_extracted_icon(&tmp);
            }
        }
        // Se copia a memoria AHORA, antes de borrar la carpeta temporal.
        meta.icon_data = icon.and_then(|path| std::fs::read(&path).ok());
        meta.icon_ext = meta
            .icon_data
            .as_deref()
            .and_then(sniff_icon_ext)
            .map(|ext| ext.to_string());
        run_extract(appimage, &tmp, "*.desktop")?;
        if let Some(desktop) = find_first_application_desktop(&tmp) {
            let text = std::fs::read_to_string(&desktop).unwrap_or_default();
            let keys = parse_desktop_file(&text);
            meta.name = keys.get("Name").cloned();
            meta.version = keys
                .get("X-AppImage-Version")
                .cloned()
                .or_else(|| keys.get("Version").cloned());
            meta.comment = keys.get("Comment").cloned();
            meta.categories = keys.get("Categories").cloned();
            meta.update_information = keys
                .get("X-AppImage-UpdateInformation")
                .or_else(|| keys.get("UpdateInformation"))
                .cloned()
                .map(|s| s.replace(['\n', '\r'], " "));
        }
        Ok(())
    })();
    // La información de actualización casi nunca está en el .desktop interno:
    // appimagetool la escribe en la sección ELF .upd_info. Se lee directamente
    // del archivo, incluso si la extracción interna falló (p. ej. AppImage
    // tipo 1 o sin .DirIcon). Si el .desktop ya la traía, no se toca.
    let has_info = meta
        .update_information
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_info {
        meta.update_information = super::updater::read_upd_info(appimage);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    meta
}

/// Detecta el formato de un icono por su contenido (cabecera), sin fiarse
/// del nombre: .DirIcon suele ser un enlace sin extensión.
pub fn sniff_icon_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some("png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start();
    if trimmed.starts_with("/* XPM */") {
        return Some("xpm");
    }
    if (trimmed.starts_with("<?xml") || trimmed.starts_with("<svg")) && trimmed.contains("<svg") {
        return Some("svg");
    }
    None
}

/// Copia el icono extraído a la carpeta de iconos de NaranjaHub.
pub fn install_icon(meta: &ExtractedMeta, icons_dir: &Path, slug: &str) -> Option<PathBuf> {
    let data = meta.icon_data.as_ref()?;
    let ext = meta.icon_ext.as_deref()?;
    if !matches!(ext, "png" | "svg" | "xpm" | "jpg" | "jpeg") {
        return None;
    }
    std::fs::create_dir_all(icons_dir).ok()?;
    let dest = icons_dir.join(format!("{slug}.{ext}"));
    std::fs::write(&dest, data).ok().map(|_| dest)
}

/// Tamaños estándar del tema hicolor (más los que suelen traer las AppImage).
const HICOLOR_SIZES: [u32; 14] = [16, 22, 24, 32, 36, 48, 64, 72, 96, 128, 192, 256, 512, 1024];

/// Tamaño estándar más cercano para la carpeta del tema hicolor (empate → el mayor).
fn nearest_hicolor_size(size: u32) -> u32 {
    let mut best = HICOLOR_SIZES[0];
    let mut best_diff = size.abs_diff(best);
    for candidate in HICOLOR_SIZES.iter().skip(1) {
        let diff = size.abs_diff(*candidate);
        if diff < best_diff || (diff == best_diff && *candidate > best) {
            best = *candidate;
            best_diff = diff;
        }
    }
    best
}

/// Tamaño (lado mayor) de un PNG leyendo solo la cabecera, desde memoria.
fn png_size(bytes: &[u8]) -> Option<u32> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let (w, h) = reader.into_dimensions().ok()?;
    Some(w.max(h))
}

/// Instala el icono en el TEMA de iconos del usuario
/// (~/.local/share/icons/hicolor/…), que es donde GNOME, KDE Plasma,
/// COSMIC y XFCE buscan los iconos de los accesos directos:
///   PNG  → hicolor/<tamaño>x<tamaño>/apps/{slug}.png
///   SVG  → hicolor/scalable/apps/{slug}.svg
/// Devuelve el nombre (slug) con el que debe referenciarse en Icon=.
pub fn install_icon_in_theme(
    data: &[u8],
    ext: &str,
    hicolor_dir: &Path,
    slug: &str,
) -> Option<String> {
    let dest_dir = match ext {
        "png" => {
            let size = png_size(data).map(nearest_hicolor_size).unwrap_or(48);
            hicolor_dir.join(format!("{size}x{size}/apps"))
        }
        "svg" => hicolor_dir.join("scalable/apps"),
        _ => return None,
    };
    std::fs::create_dir_all(&dest_dir).ok()?;
    let dest = dest_dir.join(format!("{slug}.{ext}"));
    std::fs::write(&dest, data).ok()?;
    Some(slug.to_string())
}

/// Borra los iconos instalados en el tema hicolor para un slug (al quitar
/// el acceso directo). Los nombres de carpeta usados al instalar pueden
/// ser cualquiera (tamaño real), así que se recorre el tema.
fn remove_theme_icons(hicolor_dir: &Path, slug: &str) {
    fn scan(dir: &Path, slug: &str, depth: u8) {
        if depth > 3 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan(&path, slug, depth + 1);
            } else if path
                .file_stem()
                .map(|s| s.to_string_lossy() == slug)
                .unwrap_or(false)
                && path
                    .extension()
                    .map(|e| {
                        let e = e.to_string_lossy().to_lowercase();
                        e == "png" || e == "svg"
                    })
                    .unwrap_or(false)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    scan(hicolor_dir, slug, 0);
}

fn sanitize_value(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

/// Decide de dónde saldrá el icono en Icon=: primero se instala en el tema
/// hicolor (lo más compatible con los cuatro escritorios) y si no se pudo,
/// se usa la ruta absoluta de la copia interna. Devuelve (copia interna,
/// valor de Icon=).
fn resolve_icon_reference(
    meta: &ExtractedMeta,
    icons_dir: &Path,
    hicolor_dir: &Path,
    slug: &str,
) -> (Option<PathBuf>, String) {
    let internal = install_icon(meta, icons_dir, slug);
    if let (Some(data), Some(ext)) = (meta.icon_data.as_deref(), meta.icon_ext.as_deref()) {
        if install_icon_in_theme(data, ext, hicolor_dir, slug).is_some() {
            return (internal, slug.to_string());
        }
    }
    let icon_value = match &internal {
        Some(path) => path.display().to_string(),
        None => "naranjahub".to_string(),
    };
    (internal, icon_value)
}

fn entry_text(name: &str, appimage_path: &Path, icon: &str, meta: &ExtractedMeta) -> String {
    let exec_quoted = format!("\"{}\"", appimage_path.display());
    let comment = meta
        .comment
        .clone()
        .unwrap_or_else(|| "Aplicación AppImage integrada con NaranjaHub".to_string());
    let categories = meta
        .categories
        .clone()
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "Utility;".to_string());
    let mut text = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={exec_quoted}\nTryExec={}\nIcon={icon}\nTerminal=false\nCategories={}\nStartupNotify=true\nStartupWMClass={}\n",
        sanitize_value(name),
        sanitize_value(&comment),
        appimage_path.display(),
        sanitize_value(&categories),
        super::integration::slug_for(appimage_path),
    );
    if let Some(version) = &meta.version {
        text.push_str(&format!(
            "X-NaranjaHub-Version={}\n",
            sanitize_value(version)
        ));
    }
    text.push_str(&format!(
        "X-NaranjaHub-Source={}\n",
        appimage_path.display()
    ));
    if let Some(original) = appimage_path.file_name() {
        text.push_str(&format!(
            "X-NaranjaHub-OriginalName={}\n",
            original.to_string_lossy()
        ));
    }
    if let Some(info) = &meta.update_information {
        text.push_str(&format!(
            "X-NaranjaHub-UpdateInformation={}\n",
            sanitize_value(info)
        ));
    }
    text
}

/// Busca un acceso directo previo de NaranjaHub que apunte a esta AppImage.
/// Así se evita crear duplicados tipo «naranjahub-foo (2).desktop» al
/// volver a integrar la misma aplicación.
fn find_existing_entry(desktops_dir: &Path, source: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(desktops_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name_ok = path
            .file_name()
            .map(|n| {
                let n = n.to_string_lossy();
                n.starts_with("naranjahub-") && n.ends_with(".desktop")
            })
            .unwrap_or(false);
        if !name_ok {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            let keys = parse_desktop_file(&text);
            let matches = keys
                .get("X-NaranjaHub-Source")
                .map(|s| Path::new(s.as_str()) == source)
                .unwrap_or(false);
            if matches {
                return Some(path);
            }
        }
    }
    None
}

/// Intenta refrescar la caché de aplicaciones del escritorio (mejor esfuerzo).
/// No es obligatorio: GNOME, KDE, COSMIC y XFCE vigilan la carpeta por sí
/// solos, pero acelera la aparición del icono en algunos sistemas.
pub fn refresh_desktop_database(desktops_dir: &Path) {
    let _ = Command::new("update-desktop-database")
        .arg(desktops_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Crea el acceso directo .desktop del menú de aplicaciones.
/// `icon_ref` es el valor de Icon=: normalmente el slug instalado en el
/// tema hicolor (lo ven GNOME, KDE, COSMIC y XFCE), o la ruta absoluta
/// de la copia interna si no se pudo instalar en el tema.
pub fn write_desktop_entry(
    desktops_dir: &Path,
    slug: &str,
    name: &str,
    appimage_path: &Path,
    icon_ref: &str,
    meta: &ExtractedMeta,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(desktops_dir)
        .map_err(|e| format!("no se pudo crear la carpeta de accesos: {e}"))?;
    let file_name = format!("naranjahub-{slug}.desktop");
    let dest = find_existing_entry(desktops_dir, appimage_path)
        .unwrap_or_else(|| unique_target(desktops_dir, &file_name));
    std::fs::write(&dest, entry_text(name, appimage_path, icon_ref, meta))
        .map_err(|e| format!("no se pudo crear el acceso directo: {e}"))?;
    refresh_desktop_database(desktops_dir);
    Ok(dest)
}

/// Quita el acceso directo y los iconos que instaló (copia interna e
/// iconos del tema hicolor, si son de NaranjaHub).
pub fn remove_desktop_integration(
    desktop_path: &Path,
    icons_dir: &Path,
    hicolor_dir: &Path,
) -> Result<(), String> {
    let text = std::fs::read_to_string(desktop_path).unwrap_or_default();
    let keys = parse_desktop_file(&text);
    // Slug del icono: derivado de la AppImage de origen (X-NaranjaHub-Source)
    // o, por compatibilidad con accesos antiguos, del propio Icon=.
    let slug = keys
        .get("X-NaranjaHub-Source")
        .map(|s| slug_for(Path::new(s)))
        .or_else(|| {
            keys.get("Icon").map(|icon| {
                if icon.contains('/') {
                    slug_for(Path::new(icon))
                } else {
                    utils::sanitize_filename(icon)
                }
            })
        });
    if let Some(icon) = keys.get("Icon") {
        let icon_path = PathBuf::from(icon);
        if icon_path.parent().map(|p| p == icons_dir).unwrap_or(false) {
            let _ = std::fs::remove_file(&icon_path);
        }
    }
    if let Some(slug) = slug {
        if let Ok(entries) = std::fs::read_dir(icons_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .file_stem()
                    .map(|s| s.to_string_lossy() == slug)
                    .unwrap_or(false)
                    && path.is_file()
                {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        remove_theme_icons(hicolor_dir, &slug);
    }
    std::fs::remove_file(desktop_path)
        .map_err(|e| format!("no se pudo quitar el acceso directo: {e}"))?;
    refresh_desktop_database(desktop_path.parent().unwrap_or(Path::new(".")));
    Ok(())
}

/// Re-extrae metadatos y reescribe el acceso directo (tras actualizar o analizar).
/// Usa write_desktop_entry para reutilizar el acceso existente y no crear duplicados.
/// Falla si la AppImage ya no existe: antes, «Analizar» sobre un archivo
/// borrado desde el explorador creaba un acceso directo hueco sin sentido.
pub fn refresh_integration(appimage: &Path, dirs: &DirPaths) -> Result<(), String> {
    if !appimage.is_file() {
        return Err(format!(
            "el archivo ya no existe: {}",
            appimage.display()
        ));
    }
    let meta = extract_metadata(appimage);
    let slug = slug_for(appimage);
    let name = meta
        .name
        .clone()
        .unwrap_or_else(|| discovery::display_name(appimage));
    let (_, icon_ref) = resolve_icon_reference(&meta, &dirs.icons_dir, &dirs.hicolor_dir, &slug);
    write_desktop_entry(&dirs.desktops_dir, &slug, &name, appimage, &icon_ref, &meta).map(|_| ())
}

/// Instala el archivo nuevo descargado en sustitución de la versión anterior.
///
/// - Si el asset publicado se llama igual que el archivo local, se sustituye
///   EN SITIO (respaldo previo; si algo falla se restaura y no se pierde nada,
///   y el acceso directo del menú sigue siendo válido).
/// - Si el nombre cambia, se añade el archivo nuevo, se re-integra y se
///   limpian el archivo y el acceso directo de la versión vieja.
pub fn apply_update(
    old_path: &Path,
    new_file: &Path,
    asset_name: &str,
    dirs: &DirPaths,
) -> Result<PathBuf, String> {
    if !new_file.is_file() {
        return Err(format!(
            "el archivo descargado no existe: {}",
            new_file.display()
        ));
    }
    let old_name = old_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let same_name = asset_name.eq_ignore_ascii_case(&old_name);
    let dest = if same_name {
        old_path.to_path_buf()
    } else {
        unique_target(&dirs.apps_dir, asset_name)
    };
    // Respaldo de la versión anterior antes de tocar nada.
    let mut backup: Option<PathBuf> = None;
    if dest == old_path {
        let bk = dirs
            .apps_dir
            .join(format!(".naranjahub-anterior-{}", old_name));
        let _ = std::fs::remove_file(&bk);
        std::fs::rename(old_path, &bk)
            .map_err(|e| format!("no se pudo respaldar la versión anterior: {e}"))?;
        backup = Some(bk);
    }
    let install = std::fs::rename(new_file, &dest)
        .map_err(|e| format!("no se pudo instalar: {e}"))
        .and_then(|()| set_executable(&dest))
        .and_then(|()| refresh_integration(&dest, dirs));
    match install {
        Ok(()) => {
            if let Some(bk) = &backup {
                let _ = std::fs::remove_file(bk);
            }
            if dest != old_path {
                let (map, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
                if let Some(old_desktop) = map.get(old_path).cloned() {
                    let _ = remove_desktop_integration(
                        &old_desktop,
                        &dirs.icons_dir,
                        &dirs.hicolor_dir,
                    );
                }
                let _ = std::fs::remove_file(old_path);
            }
            Ok(dest)
        }
        Err(err) => {
            // Deshacer por completo: el archivo nuevo vuelve a su ruta temporal
            // y la versión anterior se restaura intacta. Nada queda a medias.
            if backup.is_some() {
                if dest.is_file() {
                    let _ = std::fs::rename(&dest, new_file);
                }
                if let Some(bk) = &backup {
                    if !old_path.exists() {
                        let _ = std::fs::rename(bk, old_path);
                    }
                }
            }
            Err(err)
        }
    }
}

/// Flujo completo de integración: copia → permisos → icono → acceso directo.
pub fn integrate(
    src: &Path,
    options: &IntegrateOptions,
    dirs: &DirPaths,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<IntegrateOutcome, String> {
    let file_name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("ruta inválida")?;
    std::fs::create_dir_all(&dirs.apps_dir)
        .map_err(|e| format!("no se pudo crear {}: {e}", dirs.apps_dir.display()))?;

    // Si el archivo ya vive en la carpeta gestionada, se integra donde está.
    let already_managed = src.parent().map(|p| p == dirs.apps_dir).unwrap_or(false);
    let target: PathBuf = if already_managed {
        src.to_path_buf()
    } else if options.move_original {
        // Mover (renombrar) es instantáneo y no deja duplicados si falla el
        // borrado posterior; si son sistemas de archivos distintos, copia + borra.
        let dest = unique_target(&dirs.apps_dir, &file_name);
        if std::fs::rename(src, &dest).is_ok() {
            dest
        } else {
            copy_with_progress(src, &dest, progress)?;
            std::fs::remove_file(src).map_err(|e| format!("no se pudo mover el original: {e}"))?;
            dest
        }
    } else {
        let dest = unique_target(&dirs.apps_dir, &file_name);
        copy_with_progress(src, &dest, progress)?;
        dest
    };
    set_executable(&target)?;

    let meta = extract_metadata(&target);
    let slug = slug_for(&target);
    // Copia interna + instalación en el tema hicolor (Icon= por nombre).
    let (_, icon_ref) = resolve_icon_reference(&meta, &dirs.icons_dir, &dirs.hicolor_dir, &slug);
    let name = meta
        .name
        .clone()
        .unwrap_or_else(|| discovery::display_name(&target));

    let mut desktop_path = None;
    if options.create_entry {
        desktop_path = Some(write_desktop_entry(
            &dirs.desktops_dir,
            &slug,
            &name,
            &target,
            &icon_ref,
            &meta,
        )?);
    }

    Ok(IntegrateOutcome {
        new_path: target,
        desktop_path,
        app_name: name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_unicos() {
        let base = std::env::temp_dir().join(format!("nh-test-uniq-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let a = base.join("App.AppImage");
        std::fs::write(&a, "x").unwrap();
        assert_eq!(
            unique_target(&base, "App.AppImage"),
            base.join("App (2).AppImage")
        );
        std::fs::write(base.join("App (2).AppImage"), "x").unwrap();
        assert_eq!(
            unique_target(&base, "App.AppImage"),
            base.join("App (3).AppImage")
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn parseo_desktop() {
        let keys = parse_desktop_file(
            "[Desktop Action a]\nName=x\n\n[Desktop Entry]\n# comentario\nType=Application\nName=Demo App\nExec=/opt/a\n",
        );
        assert_eq!(keys.get("Type").map(String::as_str), Some("Application"));
        assert_eq!(keys.get("Name").map(String::as_str), Some("Demo App"));
        assert_eq!(keys.get("Exec").map(String::as_str), Some("/opt/a"));
        assert!(!keys.contains_key("Name=x"));
    }

    #[test]
    fn slug_seguro() {
        assert_eq!(slug_for(Path::new("/x/Mi App (1).AppImage")), "Mi_App_1");
    }

    #[test]
    fn tamano_hicolor_mas_cercano() {
        assert_eq!(nearest_hicolor_size(1), 16);
        assert_eq!(nearest_hicolor_size(48), 48);
        assert_eq!(nearest_hicolor_size(50), 48);
        assert_eq!(nearest_hicolor_size(100), 96);
        assert_eq!(nearest_hicolor_size(300), 256);
        assert_eq!(nearest_hicolor_size(2000), 1024);
    }

    /// PNG real de 1×1 (para probar la lectura de tamaño de la cabecera).
    const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn icono_instalado_en_tema_hicolor() {
        let base = std::env::temp_dir().join(format!("nh-test-icon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("icono.png");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(&src, PNG_1X1).unwrap();
        let hicolor = base.join("icons/hicolor");
        let data = std::fs::read(&src).unwrap();
        let name = install_icon_in_theme(&data, "png", &hicolor, "demo").expect("instala");
        assert_eq!(name, "demo");
        // 1x1 → carpeta 16x16
        assert!(hicolor.join("16x16/apps/demo.png").is_file());
        // Reinstalar (actualizar) no debe fallar
        install_icon_in_theme(&data, "png", &hicolor, "demo").expect("reinstala");
        // Un SVG va a scalable
        let svg = base.join("icono.svg");
        std::fs::write(&svg, "<svg xmlns=\"http://www.w3.org/2000/svg\"/>").unwrap();
        let svg_data = std::fs::read(&svg).unwrap();
        install_icon_in_theme(&svg_data, "svg", &hicolor, "demo2").expect("instala svg");
        assert!(hicolor.join("scalable/apps/demo2.svg").is_file());
        // Y se puede borrar por slug
        remove_theme_icons(&hicolor, "demo");
        assert!(!hicolor.join("16x16/apps/demo.png").exists());
        assert!(hicolor.join("scalable/apps/demo2.svg").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Prepara un entorno de actualización: AppImage "antigua" ya integrada.
    fn setup_update_env(tag: &str) -> (PathBuf, DirPaths) {
        let base = std::env::temp_dir().join(format!("nh-test-upd-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dirs = DirPaths {
            apps_dir: base.join("gestion"),
            icons_dir: base.join("iconos"),
            hicolor_dir: base.join("hicolor"),
            desktops_dir: base.join("accesos"),
        };
        std::fs::create_dir_all(&base).unwrap();
        let old = base.join("origen").join("DemoApp-1.0.0-x86_64.AppImage");
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, "contenido-v1").unwrap();
        set_executable(&old).unwrap();
        let outcome = integrate(&old, &IntegrateOptions::default(), &dirs, &|_, _| {}).unwrap();
        assert!(outcome.desktop_path.is_some());
        (base, dirs)
    }

    #[test]
    fn actualizacion_en_sitio_mismo_nombre() {
        let (base, dirs) = setup_update_env("mismo");
        let old = dirs.apps_dir.join("DemoApp-1.0.0-x86_64.AppImage");
        assert!(old.is_file(), "la copia integrada debe existir");
        let nuevo = base.join("descarga.part");
        std::fs::write(&nuevo, "contenido-v2").unwrap();
        let dest =
            apply_update(&old, &nuevo, "DemoApp-1.0.0-x86_64.AppImage", &dirs).expect("actualiza");
        // Sustitución en sitio: misma ruta, contenido nuevo, sin residuos.
        assert_eq!(dest, old);
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "contenido-v2");
        assert!(!base
            .join(".naranjahub-anterior-DemoApp-1.0.0-x86_64.AppImage")
            .exists());
        // El acceso directo sigue apuntando al mismo archivo.
        let (map, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
        assert!(
            map.contains_key(&dest),
            "el acceso directo debe seguir válido"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn actualizacion_con_nombre_nuevo() {
        let (base, dirs) = setup_update_env("nuevo");
        let old = dirs.apps_dir.join("DemoApp-1.0.0-x86_64.AppImage");
        let (old_desktop, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
        let old_desktop = old_desktop.get(&old).cloned().expect("acceso previo");
        let nuevo = base.join("descarga.part");
        std::fs::write(&nuevo, "contenido-v2").unwrap();
        let dest =
            apply_update(&old, &nuevo, "DemoApp-1.1.0-x86_64.AppImage", &dirs).expect("actualiza");
        // Archivo nuevo instalado y viejo eliminado con su acceso directo.
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "contenido-v2");
        assert!(!old.exists());
        assert!(!old_desktop.exists(), "el acceso del viejo debe quitarse");
        let (map, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
        assert!(map.contains_key(&dest), "el nuevo debe quedar integrado");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn actualizacion_fallida_restaura_la_version_anterior() {
        let (base, dirs) = setup_update_env("fallo");
        let old = dirs.apps_dir.join("DemoApp-1.0.0-x86_64.AppImage");
        // La reintegración fallará porque el acceso directo existente pasa a
        // ser de solo lectura (fs::write no puede sobrescribirlo).
        let mut desktop_viejo = None;
        if let Ok(entries) = std::fs::read_dir(&dirs.desktops_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("naranjahub-") && name.ends_with(".desktop") {
                    desktop_viejo = Some(entry.path());
                }
            }
        }
        let desktop_viejo = desktop_viejo.expect("acceso previo");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&desktop_viejo, std::fs::Permissions::from_mode(0o444))
                .unwrap();
        }
        let nuevo = base.join("descarga.part");
        std::fs::write(&nuevo, "contenido-v2").unwrap();
        let result = apply_update(&old, &nuevo, "DemoApp-1.0.0-x86_64.AppImage", &dirs);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(&desktop_viejo, std::fs::Permissions::from_mode(0o644));
        }
        assert!(result.is_err(), "debe fallar al no poder reintegrar");
        // La versión anterior se restaura intacta.
        assert!(old.is_file(), "el archivo original debe seguir existiendo");
        assert_eq!(std::fs::read_to_string(&old).unwrap(), "contenido-v1");
        // Y el archivo nuevo descargado no sustituyó al viejo.
        assert!(!base
            .join(".naranjahub-anterior-DemoApp-1.0.0-x86_64.AppImage")
            .exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
