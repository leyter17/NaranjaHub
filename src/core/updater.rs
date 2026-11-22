// NaranjaHub — comprobación de actualizaciones vía GitHub Releases.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// Se basa en la convención gh-releases-zsync|owner|repo|tag|patrón que las
// AppImage declaran en sus metadatos. Esa información NO suele estar en el
// .desktop interno: appimagetool la escribe en la sección ELF `.upd_info`
// del ejecutable, así que se lee directamente ahí (sin herramientas externas).

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const USER_AGENT: &str = concat!(
    "NaranjaHub/",
    env!("CARGO_PKG_VERSION"),
    " (AppImage manager; GPL v3)"
);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GhUpdateInfo {
    pub owner: String,
    pub repo: String,
    pub pattern: String,
}

/// Parsea "gh-releases-zsync|owner|repo|tag|patrón".
pub fn parse_update_information(raw: &str) -> Option<GhUpdateInfo> {
    let rest = raw.trim().strip_prefix("gh-releases-zsync|")?;
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() < 4 {
        return None;
    }
    let owner = parts[0].trim().to_string();
    let repo = parts[1].trim().to_string();
    let pattern = parts[3].trim().to_string();
    if owner.is_empty() || repo.is_empty() || pattern.is_empty() {
        return None;
    }
    Some(GhUpdateInfo {
        owner,
        repo,
        pattern,
    })
}

/// Comparación con comodines '*' y '?', sin distinguir mayúsculas.
pub fn match_wildcard(text: &str, pattern: &str) -> bool {
    let text = text.to_lowercase();
    let pattern = pattern.to_lowercase();
    wildcard_iter(text.as_bytes(), pattern.as_bytes())
}

fn wildcard_iter(text: &[u8], pat: &[u8]) -> bool {
    let (mut ti, mut pi) = (0usize, 0usize);
    let (mut star_t, mut star_p) = (0usize, 0usize);
    let mut have_star = false;
    while ti < text.len() {
        if pi < pat.len() && (pat[pi] == b'?' || pat[pi] == text[ti]) {
            ti += 1;
            pi += 1;
        } else if pi < pat.len() && pat[pi] == b'*' {
            star_t = ti;
            star_p = pi;
            have_star = true;
            pi += 1;
        } else if have_star {
            ti = star_t + 1;
            star_t += 1;
            pi = star_p + 1;
        } else {
            return false;
        }
    }
    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }
    pi == pat.len()
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

const NOISE_SUFFIXES: [&str; 6] = [".blockmap", ".zsync", ".sha256", ".sha512", ".sig", ".asc"];

/// Primer asset que coincide con el patrón, ignorando blockmaps y firmas.
pub fn find_matching_asset(assets: &[ReleaseAsset], pattern: &str) -> Option<ReleaseAsset> {
    assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_lowercase();
            !NOISE_SUFFIXES.iter().any(|s| name.ends_with(s))
                && match_wildcard(&asset.name, pattern)
        })
        .cloned()
}

pub fn new_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(120))
        .build()
}

/// Agente SOLO para comprobaciones de versión (peticiones pequeñas a la API
/// de GitHub): conecta en 8 s y caduca la petición entera en 30 s. Antes se
/// usaba el agente de descargas y, sin conexión, la resolución DNS podía
/// bloquearse mucho tiempo: la ficha se quedaba eternamente en
/// «Comprobando...». Este límite global, junto con el vigilante de trabajos
/// de la interfaz, garantiza que el estado nunca quede colgado.
pub fn new_check_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(20))
        .timeout(Duration::from_secs(30))
        .build()
}

/// Traduce un error de red a un mensaje claro. Los fallos de DNS, conexión
/// o E/S se presentan como «sin conexión a internet», que es lo que el
/// usuario entiende cuando no hay red en lugar de un texto técnico críptico.
pub fn friendly_error(err: &ureq::Error) -> String {
    if let ureq::Error::Transport(transport) = err {
        if is_offline_kind(transport.kind()) {
            return format!("sin conexión a internet ({transport})");
        }
    }
    err.to_string()
}

/// ¿Es un tipo de error de red que indica que no hay conexión con el
/// servidor (falló la resolución DNS, la conexión TCP o la E/S de red)?
pub fn is_offline_kind(kind: ureq::ErrorKind) -> bool {
    matches!(
        kind,
        ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed | ureq::ErrorKind::Io
    )
}

// ---------- lectura de la sección ELF `.upd_info` ----------
//
// Las AppImage tipo 2 guardan la información de actualización en una sección
// ELF llamada `.upd_info` (así la escribe `appimagetool --updateinformation`
// y así la lee AppImageUpdate). Se parsea el ELF a mano, con lecturas por
// posición (seek) para NO cargar en memoria AppImage de cientos de MB: solo
// se leen la cabecera, la tabla de secciones, la tabla de nombres y los
// bytes de la sección pedida. Soporta ELF 32 y 64 bits little-endian.

fn le_u16(bytes: &[u8], pos: usize) -> Option<u16> {
    if pos + 2 <= bytes.len() {
        Some(u16::from_le_bytes([bytes[pos], bytes[pos + 1]]))
    } else {
        None
    }
}

fn le_u32(bytes: &[u8], pos: usize) -> Option<u32> {
    if pos + 4 <= bytes.len() {
        Some(u32::from_le_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
        ]))
    } else {
        None
    }
}

fn le_u64(bytes: &[u8], pos: usize) -> Option<u64> {
    if pos + 8 <= bytes.len() {
        Some(u64::from_le_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]))
    } else {
        None
    }
}

/// Entradas mínimas de la tabla de secciones de un ELF.
struct SectionTable {
    /// (sh_name, sh_offset, sh_size) de cada sección.
    entries: Vec<(u32, u64, u64)>,
}

/// Descripción de la geometría ELF sacada de la cabecera (64 bytes).
struct ElfHeader {
    wide: bool,
    shoff: u64,
    shentsize: u64,
    shnum: u64,
    shstrndx: u64,
}

fn parse_elf_header(header: &[u8; 64]) -> Option<ElfHeader> {
    if &header[0..4] != b"\x7fELF" || header[5] != 1 {
        return None; // no es ELF o no es little-endian
    }
    let (wide, shoff, shentsize, shnum, shstrndx) = match header[4] {
        1 => (
            false,
            le_u32(header, 32)? as u64,
            le_u16(header, 46)? as u64,
            le_u16(header, 48)? as u64,
            le_u16(header, 50)? as u64,
        ),
        2 => (
            true,
            le_u64(header, 40)?,
            le_u16(header, 58)? as u64,
            le_u16(header, 60)? as u64,
            le_u16(header, 62)? as u64,
        ),
        _ => return None,
    };
    let min_entry = if wide { 64 } else { 40 };
    if shnum == 0 || shstrndx == 0xffff || shentsize < min_entry {
        return None;
    }
    // Límite sensato de entradas para no asignar memoria absurda con un
    // archivo corrupto (una tabla normal tiene decenas de secciones).
    if shnum > 65_536 {
        return None;
    }
    Some(ElfHeader {
        wide,
        shoff,
        shentsize,
        shnum,
        shstrndx,
    })
}

/// Lee la tabla de secciones completa desde el archivo (unos pocos KB).
fn read_section_table(file: &mut std::fs::File, header: &ElfHeader) -> Option<SectionTable> {
    use std::io::{Read, Seek, SeekFrom};
    let total = header
        .shnum
        .checked_mul(header.shentsize)
        .filter(|t| *t <= 4 * 1024 * 1024)?;
    file.seek(SeekFrom::Start(header.shoff)).ok()?;
    let mut table_bytes = vec![0u8; total as usize];
    file.read_exact(&mut table_bytes).ok()?;
    let (offset_pos, size_pos) = if header.wide { (24, 32) } else { (16, 20) };
    let mut entries = Vec::with_capacity(header.shnum as usize);
    for index in 0..header.shnum as usize {
        let base = index * header.shentsize as usize;
        let name = le_u32(&table_bytes, base)?;
        let offset = if header.wide {
            le_u64(&table_bytes, base + offset_pos)?
        } else {
            le_u32(&table_bytes, base + offset_pos)? as u64
        };
        let size = if header.wide {
            le_u64(&table_bytes, base + size_pos)?
        } else {
            le_u32(&table_bytes, base + size_pos)? as u64
        };
        entries.push((name, offset, size));
    }
    Some(SectionTable { entries })
}

/// Nombre de una sección a partir de la tabla de nombres (.shstrtab).
fn name_from_strtab(strtab: &[u8], sh_name: u32) -> Option<String> {
    let name_bytes = strtab.get(sh_name as usize..)?;
    let len = name_bytes.iter().position(|&b| b == 0).unwrap_or(0);
    Some(String::from_utf8_lossy(&name_bytes[..len]).to_string())
}

/// Lee el contenido textual de una sección ELF por su nombre, con seeks.
fn read_elf_section(path: &Path, wanted: &str) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 64];
    file.read_exact(&mut header).ok()?;
    let elf = parse_elf_header(&header)?;
    let table = read_section_table(&mut file, &elf)?;
    // Tabla de nombres de sección.
    let (_, strtab_offset, strtab_size) = *table.entries.get(elf.shstrndx as usize)?;
    if strtab_size > 4 * 1024 * 1024 {
        return None;
    }
    file.seek(SeekFrom::Start(strtab_offset)).ok()?;
    let mut strtab = vec![0u8; strtab_size as usize];
    file.read_exact(&mut strtab).ok()?;
    // Buscar la sección pedida y leer SOLO sus bytes.
    for (sh_name, offset, size) in &table.entries {
        if name_from_strtab(&strtab, *sh_name)?.as_str() != wanted {
            continue;
        }
        if *size > 4096 {
            // .upd_info es una cadena corta; un tamaño absurdo indica corrupción.
            continue;
        }
        if *size == 0 {
            return Some(String::new());
        }
        file.seek(SeekFrom::Start(*offset)).ok()?;
        let mut data = vec![0u8; *size as usize];
        file.read_exact(&mut data).ok()?;
        let text_len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        return Some(String::from_utf8_lossy(&data[..text_len]).to_string());
    }
    None
}

/// Información de actualización embebida en una AppImage tipo 2 (sección ELF
/// `.upd_info`). Devuelve None si el archivo no es ELF o no lleva información.
pub fn read_upd_info(appimage: &Path) -> Option<String> {
    let raw = read_elf_section(appimage, ".upd_info")?;
    let text = raw.trim().to_string();
    if text.is_empty() {
        return None;
    }
    Some(text)
}

/// Construye un ELF64 de prueba con una sección de texto arbitraria.
/// Solo para las simulaciones y pruebas unitarias (no se usa en la app).
#[doc(hidden)]
pub fn build_test_elf(section_name: &str, section_data: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 64 + 3 * 64 + 64];
    // Cabecera ELF64 little-endian.
    out[0..4].copy_from_slice(b"\x7fELF");
    out[4] = 2; // clase: 64 bits
    out[5] = 1; // datos: little-endian
    out[6] = 1; // versión
    out[40..48].copy_from_slice(&64u64.to_le_bytes()); // e_shoff
    out[58..60].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
    out[60..62].copy_from_slice(&3u16.to_le_bytes()); // e_shnum
    out[62..64].copy_from_slice(&1u16.to_le_bytes()); // e_shstrndx (índice de .shstrtab)
                                                      // Tabla de secciones en 64: NULL, .shstrtab, la sección pedida.
    let mut shstrtab: Vec<u8> = vec![0];
    shstrtab.extend_from_slice(b".shstrtab\0");
    let wanted_off = shstrtab.len() as u32;
    shstrtab.extend_from_slice(section_name.as_bytes());
    shstrtab.push(0);
    let strtab_off = out.len() as u64; // se coloca al final del archivo
                                       // Entrada .shstrtab
    let entry = |name: u32, offset: u64, size: u64| -> Vec<u8> {
        let mut e = vec![0u8; 64];
        e[0..4].copy_from_slice(&name.to_le_bytes());
        e[24..32].copy_from_slice(&offset.to_le_bytes());
        e[32..40].copy_from_slice(&size.to_le_bytes());
        e
    };
    out[64..128].copy_from_slice(&entry(0, 0, 0));
    out[128..192].copy_from_slice(&entry(1, strtab_off, shstrtab.len() as u64));
    let data_off = strtab_off + shstrtab.len() as u64;
    out[192..256].copy_from_slice(&entry(wanted_off, data_off, section_data.len() as u64));
    out.extend_from_slice(&shstrtab);
    out.extend_from_slice(section_data);
    out
}

// ---------- comparación de versiones ----------

/// Busca un número de versión tipo "1.2.3" en un texto (nombre de archivo,
/// nombre de asset...). Exige al menos un punto para no confundir "x86_64".
pub fn extract_version(text: &str) -> Option<Vec<u32>> {
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();
    let mut best: Option<Vec<u32>> = None;
    let mut start: Option<usize> = None;
    for (pos, &ch) in bytes.iter().chain(std::iter::once(&b' ')).enumerate() {
        let in_block = ch.is_ascii_digit() || ch == b'.';
        match (start, in_block) {
            (None, true) => start = Some(pos),
            (Some(begin), false) => {
                start = None;
                // El punto cuenta como parte del bloque (1.2.3 en 1.2.3.tar.gz),
                // así que se recortan los puntos del final antes de parsear.
                let block = lower[begin..pos].trim_end_matches('.');
                if block.contains('.') {
                    if let Some(v) = parse_version(block) {
                        // El candidato con más segmentos gana (1.2.3 sobre 1.2).
                        if best.as_ref().map_or(true, |b| b.len() < v.len()) {
                            best = Some(v);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    best
}

fn parse_version(block: &str) -> Option<Vec<u32>> {
    let parts: Option<Vec<u32>> = block.split('.').map(|s| s.parse::<u32>().ok()).collect();
    parts.filter(|v| !v.is_empty())
}

/// Compara dos versiones numéricas: 1.2.10 > 1.2.9, 1.2 > 1.1.9, 1.0 = 1.0.0.
pub fn compare_versions(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    for (x, y) in a.iter().zip(b.iter()) {
        match x.cmp(y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    let trim_zeros = |v: &[u32]| v.len() - v.iter().rev().take_while(|&&x| x == 0).count();
    trim_zeros(a).cmp(&trim_zeros(b))
}

/// ¿Hay versión nueva? `asset_name` es el archivo publicado, `current_version`
/// la versión instalada (si se conoce) y `local_name` el nombre del archivo
/// local. Sin versiones comparables se cae a la comparación por nombre exacto.
pub fn is_newer_asset(asset_name: &str, current_version: Option<&str>, local_name: &str) -> bool {
    let asset_v = extract_version(asset_name);
    let current_v = current_version
        .and_then(extract_version)
        .or_else(|| extract_version(local_name));
    match (asset_v, current_v) {
        (Some(new), Some(old)) => compare_versions(&new, &old) == std::cmp::Ordering::Greater,
        _ => !asset_name.eq_ignore_ascii_case(local_name),
    }
}

/// Consulta la última versión publicada en GitHub y devuelve el asset que
/// coincide con el patrón de la AppImage.
pub fn check_latest(agent: &ureq::Agent, info: &GhUpdateInfo) -> Result<ReleaseAsset, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        info.owner, info.repo
    );
    let response = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => {
                if code == 403 {
                    "GitHub devolvió 403 (posible límite de peticiones)".to_string()
                } else if code == 404 {
                    "el repositorio o las versiones no existen en GitHub".to_string()
                } else {
                    format!("GitHub respondió con código {code}")
                }
            }
            other => friendly_error(&other),
        })?;
    let value: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("respuesta inválida de GitHub: {e}"))?;
    let assets_json = value
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or("GitHub no devolvió la lista de archivos")?;
    let assets: Vec<ReleaseAsset> = assets_json
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let url = a.get("browser_download_url")?.as_str()?.to_string();
            let size = a.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            Some(ReleaseAsset { name, url, size })
        })
        .collect();
    find_matching_asset(&assets, &info.pattern)
        .ok_or_else(|| format!("ningún archivo coincide con el patrón «{}»", info.pattern))
}

/// Descarga un archivo con informes de progreso (bytes, total).
pub fn download(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<(), String> {
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("no se pudo descargar: {e}"))?;
    let total = response
        .all("content-length")
        .first()
        .and_then(|v| v.parse::<u64>().ok());
    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("no se pudo crear {}: {e}", dest.display()))?;
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 65_536];
    let mut got: u64 = 0;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("error durante la descarga: {e}"))?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buffer[..read])
            .map_err(|e| format!("error al guardar la descarga: {e}"))?;
        got += read as u64;
        progress(got, total);
    }
    if let Some(total) = total {
        if got < total {
            return Err("descarga incompleta".to_string());
        }
    }
    Ok(())
}

// ---------- dirección de actualización definida a mano ----------
//
// Muchas AppImage no llevan la información de actualización embebida, así
// que el usuario puede escribir la dirección a mano en la pestaña
// Actualizaciones. Se aceptan tres formas: la cadena completa
// «gh-releases-zsync|usuario|repo|etiqueta|patrón», la URL del repositorio
// (https://github.com/usuario/repo) o solo «usuario/repo». En los dos
// últimos casos el patrón se deduce del nombre del archivo local.

/// Deduce un patrón de asset a partir del nombre local del archivo:
/// sustituye el primer bloque de versión (dígitos y puntos, con al menos
/// un dígito y un punto) por «*». Ej.: «MiApp-1.2.0-x86_64.AppImage» →
/// «MiApp-*-x86_64.AppImage». Sin bloque de versión, el nombre queda igual
/// (y «x86_64» no se confunde con una versión porque no lleva punto).
pub fn derive_pattern(local_name: &str) -> String {
    let bytes = local_name.as_bytes();
    let mut out = String::with_capacity(local_name.len() + 1);
    let mut copied = 0usize; // posición hasta la que ya se copió tal cual
    let mut start: Option<usize> = None;
    let mut replaced = false;
    // El espacio centinela del final cierra el último bloque.
    for (pos, &ch) in bytes.iter().chain(std::iter::once(&b' ')).enumerate() {
        let in_block = ch.is_ascii_digit() || ch == b'.';
        match (start, in_block) {
            (None, true) => start = Some(pos),
            (Some(begin), false) => {
                start = None;
                let block = &local_name[begin..pos];
                // Como extract_version: los puntos del final son el
                // separador nombre/extensión, no parte de la versión.
                let trimmed = block.trim_end_matches('.');
                let is_version = !replaced
                    && trimmed.contains('.')
                    && trimmed.bytes().any(|b| b.is_ascii_digit());
                out.push_str(&local_name[copied..begin]);
                copied = pos;
                if is_version {
                    out.push('*');
                    // Los puntos finales se conservan (separador de extensión).
                    out.push_str(&block[trimmed.len()..]);
                    replaced = true;
                } else {
                    out.push_str(block);
                }
            }
            _ => {}
        }
    }
    if !replaced {
        return local_name.to_string();
    }
    out.push_str(&local_name[copied..]);
    out
}

/// Interpreta la dirección de actualización escrita a mano y la convierte
/// en información de GitHub. Devuelve un error con el motivo si no se
/// puede reconocer la dirección.
pub fn parse_manual_address(raw: &str, local_name: &str) -> Result<GhUpdateInfo, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("la dirección está vacía".to_string());
    }
    if text.starts_with("gh-releases-zsync|") {
        return parse_update_information(text).ok_or_else(|| {
            "la cadena está incompleta: gh-releases-zsync|usuario|repo|etiqueta|patrón".to_string()
        });
    }
    // Quitar el protocolo y el dominio de GitHub si vienen.
    let mut rest = text.to_string();
    let lower = rest.to_lowercase();
    for prefix in ["https://", "http://"] {
        if lower.starts_with(prefix) {
            rest = rest[prefix.len()..].to_string();
            break;
        }
    }
    let lower = rest.to_lowercase();
    for host in ["www.github.com/", "github.com/"] {
        if lower.starts_with(host) {
            rest = rest[host.len()..].to_string();
            break;
        }
    }
    // Lo que quede debe empezar por usuario/repo (el resto de segmentos,
    // como /releases o /tree/main, se ignoran).
    let mut segments = rest.split('/').filter(|s| !s.is_empty());
    let owner = segments.next().unwrap_or_default().trim().to_string();
    let repo = segments
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches(".git")
        .to_string();
    if owner.is_empty() || repo.is_empty() {
        return Err("no se reconoció el repositorio; usa https://github.com/usuario/repo, usuario/repo o la cadena gh-releases-zsync completa".to_string());
    }
    Ok(GhUpdateInfo {
        owner,
        repo,
        pattern: derive_pattern(local_name),
    })
}

/// Cadena canónica gh-releases-zsync a partir de la información parseada.
/// La etiqueta siempre es «latest», que es la que consulta la API de GitHub.
pub fn canonical_update_information(info: &GhUpdateInfo) -> String {
    format!(
        "gh-releases-zsync|{}|{}|latest|{}",
        info.owner, info.repo, info.pattern
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errores_friendly_sin_conexion() {
        // Los tipos de fallo de DNS / conexión / E/S se marcan como «sin conexión».
        assert!(is_offline_kind(ureq::ErrorKind::Dns));
        assert!(is_offline_kind(ureq::ErrorKind::ConnectionFailed));
        assert!(is_offline_kind(ureq::ErrorKind::Io));
        // Otros fallos de transporte (p. ej. BadStatus) no se alteran.
        assert!(!is_offline_kind(ureq::ErrorKind::BadStatus));
        assert!(!is_offline_kind(ureq::ErrorKind::InvalidUrl));
        // Un error de estado HTTP no se traduce a «sin conexión».
        let status = friendly_error(&ureq::Error::Status(
            403,
            ureq::Response::new(403, "Forbidden", "").unwrap(),
        ));
        assert!(!status.starts_with("sin conexión"), "{status}");
        assert!(status.contains("403"), "{status}");
    }

    #[test]
    fn patron_deducido_del_nombre() {
        assert_eq!(
            derive_pattern("MiApp-1.2.0-x86_64.AppImage"),
            "MiApp-*-x86_64.AppImage"
        );
        assert_eq!(derive_pattern("MiApp-2.0"), "MiApp-*");
        // Sin bloque de versión, el nombre queda igual.
        assert_eq!(derive_pattern("Programa.AppImage"), "Programa.AppImage");
        // La arquitectura x86_64 no es una versión (no lleva punto).
        assert_eq!(
            derive_pattern("Demo-x86_64.AppImage"),
            "Demo-x86_64.AppImage"
        );
        // Fechas como versión (el punto final se conserva como separador).
        assert_eq!(derive_pattern("app-2024.01.AppImage"), "app-*.AppImage");
        // Versión a final de nombre sin extensión.
        assert_eq!(derive_pattern("1.2.3-MiApp"), "*-MiApp");
    }

    #[test]
    fn direccion_manual() {
        // URL del repositorio.
        let info = parse_manual_address(
            "https://github.com/usuario/repositorio",
            "MiApp-1.2.0-x86_64.AppImage",
        )
        .expect("URL válida");
        assert_eq!(info.owner, "usuario");
        assert_eq!(info.repo, "repositorio");
        assert_eq!(info.pattern, "MiApp-*-x86_64.AppImage");
        // usuario/repo a secas.
        let info = parse_manual_address("usuario/otro", "App.AppImage").expect("user/repo");
        assert_eq!(info.owner, "usuario");
        assert_eq!(info.repo, "otro");
        assert_eq!(info.pattern, "App.AppImage");
        // URL con segmentos extra (/releases, /tree...).
        let info = parse_manual_address("https://github.com/u/r/releases/latest", "A-1.0.AppImage")
            .expect("URL larga");
        assert_eq!(info.repo, "r");
        assert_eq!(info.pattern, "A-*.AppImage");
        // Cadena completa: mantiene su propio patrón.
        let info = parse_manual_address(
            "gh-releases-zsync|u|r|latest|Mi-*.AppImage",
            "Demo.AppImage",
        )
        .expect("zsync");
        assert_eq!(info.pattern, "Mi-*.AppImage");
        // Cadena canónica ida y vuelta.
        let canon = canonical_update_information(&info);
        assert_eq!(canon, "gh-releases-zsync|u|r|latest|Mi-*.AppImage");
        assert!(parse_update_information(&canon).is_some());
        // Direcciones inválidas.
        assert!(parse_manual_address("", "x.AppImage").is_err());
        assert!(parse_manual_address("   ", "x.AppImage").is_err());
        assert!(parse_manual_address("https://github.com/solo-owner", "x.AppImage").is_err());
        assert!(parse_manual_address("no-es-una-direccion", "x.AppImage").is_err());
        assert!(parse_manual_address("gh-releases-zsync|incompleta", "x.AppImage").is_err());
    }

    #[test]
    fn parseo_zsync() {
        let info = parse_update_information("gh-releases-zsync|proyecto|repo|latest|*.AppImage")
            .expect("debe parsear");
        assert_eq!(info.owner, "proyecto");
        assert_eq!(info.repo, "repo");
        assert_eq!(info.pattern, "*.AppImage");
        assert!(parse_update_information("otro-formato|x|y|z").is_none());
        assert!(parse_update_information("gh-releases-zsync||repo|latest|*").is_none());
    }

    #[test]
    fn comodines() {
        assert!(match_wildcard("Foo-1.2.AppImage", "*.AppImage"));
        assert!(match_wildcard("foo.tar.gz", "FOO*.GZ"));
        assert!(match_wildcard("appimage", "a*image"));
        assert!(match_wildcard("abc", "?bc"));
        assert!(!match_wildcard("Foo-1.2.AppImage", "Foo-?.AppImage"));
        assert!(!match_wildcard("abc", "abcd"));
        assert!(!match_wildcard("abc", "ab"));
    }

    #[test]
    fn seleccion_de_assets() {
        let assets = vec![
            ReleaseAsset {
                name: "Demo-1.2.0.AppImage".into(),
                url: "u1".into(),
                size: 1,
            },
            ReleaseAsset {
                name: "Demo-1.2.0.AppImage.blockmap".into(),
                url: "u2".into(),
                size: 1,
            },
            ReleaseAsset {
                name: "fuentes.tar.gz".into(),
                url: "u3".into(),
                size: 2,
            },
        ];
        let found = find_matching_asset(&assets, "Demo-*.AppImage").expect("debe encontrar");
        assert_eq!(found.name, "Demo-1.2.0.AppImage");
        assert!(find_matching_asset(&assets, "NoExiste-*.AppImage").is_none());
    }

    #[test]
    fn seccion_upd_info_de_elf() {
        use std::io::Write;
        let base = std::env::temp_dir().join(format!("nh-test-elf-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let path = base.join("App.AppImage");
        let info = "gh-releases-zsync|autor|repositorio|latest|MiApp-*.AppImage";
        let elf = build_test_elf(".upd_info", format!("{info}\0").as_bytes());
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&elf).unwrap();
        assert_eq!(read_upd_info(&path).as_deref(), Some(info));
        // Sección vacía → None
        let empty = base.join("vacio.AppImage");
        let mut f2 = std::fs::File::create(&empty).unwrap();
        f2.write_all(&build_test_elf(".upd_info", b"\0")).unwrap();
        assert!(read_upd_info(&empty).is_none());
        // Sin sección → None
        let none = base.join("nada.AppImage");
        let mut f3 = std::fs::File::create(&none).unwrap();
        f3.write_all(&build_test_elf(".otra", b"texto")).unwrap();
        assert!(read_upd_info(&none).is_none());
        // No es ELF → None
        let no_elf = base.join("script.sh");
        std::fs::write(&no_elf, "#!/bin/sh\necho hola\n").unwrap();
        assert!(read_upd_info(&no_elf).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn extraccion_de_versiones() {
        assert_eq!(
            extract_version("MiApp-1.2.3-x86_64.AppImage"),
            Some(vec![1, 2, 3])
        );
        assert_eq!(
            extract_version("Foo-10.20.30.tar.gz"),
            Some(vec![10, 20, 30])
        );
        assert_eq!(extract_version("v2.0"), Some(vec![2, 0]));
        assert_eq!(
            extract_version("x86_64"),
            None,
            "no debe confundir la arquitectura"
        );
        assert_eq!(extract_version("sin-version"), None);
        assert_eq!(extract_version("2024.01"), Some(vec![2024, 1]));
    }

    #[test]
    fn comparacion_de_versiones() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions(&[1, 2, 10], &[1, 2, 9]), Ordering::Greater);
        assert_eq!(compare_versions(&[1, 2], &[1, 1, 9]), Ordering::Greater);
        assert_eq!(compare_versions(&[1, 0], &[1, 0, 0]), Ordering::Equal);
        assert_eq!(compare_versions(&[2, 0], &[10, 0]), Ordering::Less);
    }

    #[test]
    fn deteccion_de_version_nueva() {
        // Misma versión con distinto nombre de archivo → al día.
        assert!(!is_newer_asset(
            "MiApp-1.2.0-x86_64.AppImage",
            Some("1.2.0"),
            "MiApp (2).AppImage"
        ));
        // Versión superior → actualizar.
        assert!(is_newer_asset(
            "MiApp-1.3.0-x86_64.AppImage",
            Some("1.2.0"),
            "MiApp-1.2.0-x86_64.AppImage"
        ));
        // Versión inferior (el patrón coincide pero es más vieja) → al día.
        assert!(!is_newer_asset(
            "MiApp-1.1.0-x86_64.AppImage",
            Some("1.2.0"),
            "MiApp-1.2.0-x86_64.AppImage"
        ));
        // Sin versiones: cae a comparación de nombres.
        assert!(is_newer_asset("Otro.AppImage", None, "MiApp.AppImage"));
        assert!(!is_newer_asset("MiApp.AppImage", None, "MiApp.AppImage"));
        // La versión puede salir del propio nombre del archivo local.
        assert!(!is_newer_asset(
            "MiApp-1.2.0-x86_64.AppImage",
            None,
            "MiApp-1.2.0-x86_64.AppImage"
        ));
    }
}

#[cfg(test)]
mod pruebas_con_red {
    use super::*;

    /// PRUEBA CON INTERNET: la comprobación real contra GitHub funciona y
    /// encuentra un asset que coincide con el patrón. Se ejecuta solo con
    /// `cargo test -- --ignored`; las simulaciones --selftest siguen sin red.
    #[test]
    #[ignore = "requiere internet"]
    fn comprobacion_real_en_github() {
        let agent = new_check_agent();
        let info = GhUpdateInfo {
            owner: "TheAssassin".to_string(),
            repo: "AppImageLauncher".to_string(),
            pattern: "*x86_64.AppImage".to_string(),
        };
        let asset = check_latest(&agent, &info).expect("comprobación en vivo");
        assert!(asset.name.ends_with(".AppImage"), "{}", asset.name);
        assert!(asset.url.starts_with("https://"));
    }

    /// PRUEBA SIN CONEXIÓN SIMULADA: una IP no enrutable no responde al
    /// SYN (como cuando no hay internet y el DNS/red se cuelgan). El
    /// agente de comprobación DEBE caducar solo (límite global de 30 s)
    /// y el error debe clasificarse como «sin conexión a internet». Antes
    /// de este arreglo, sin este límite el estado «Comprobando...» podía
    /// quedarse bloqueado indefinidamente.
    #[test]
    #[ignore = "necesita una red que descarte paquetes (p. ej. sin internet)"]
    fn agente_de_comprobacion_caduca_sin_conexion() {
        let agent = new_check_agent();
        let started = std::time::Instant::now();
        // 10.255.255.1 es una IP privada no enrutable: la conexión se cuelga.
        let result = agent.get("https://10.255.255.1/repos/x/y/releases/latest").call();
        let elapsed = started.elapsed();
        // Caduca sola en menos de 40 s (connect 8 s + límite global 30 s).
        assert!(elapsed < std::time::Duration::from_secs(40), "{elapsed:?}");
        match result {
            Err(ureq::Error::Transport(transport)) => {
                let texto = friendly_error(&ureq::Error::Transport(transport));
                assert!(texto.starts_with("sin conexión a internet"), "{texto}");
            }
            other => panic!("se esperaba un error de transporte, salió: {other:?}"),
        }
    }
}
