// NaranjaHub — utilidades compartidas: tamaños legibles, fechas, plantillas.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use crate::i18n::Lang;

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Tamaño legible para personas: 999 B, 12.4 MB, 1.0 GB...
pub fn human_size(bytes: u64) -> String {
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    const K: f64 = 1000.0;
    let units = ["KB", "MB", "GB", "TB"];
    let mut value = bytes as f64 / K;
    let mut index = 0usize;
    while value >= K && index < units.len() - 1 {
        value /= K;
        index += 1;
    }
    format!("{value:.1} {}", units[index])
}

/// Convierte días desde la época a (año, mes, día). Algoritmo civil de Howard Hinnant.
pub fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m as u32, d as u32)
}

pub fn epoch_to_ymd(secs: u64) -> (i32, u32, u32) {
    days_to_ymd((secs / 86_400) as i64)
}

const MONTHS_ES: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];
const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Fecha legible: "21 nov 2026" (es) o "Nov 21, 2026" (en).
pub fn fmt_date(epoch_secs: u64, lang: Lang) -> String {
    let (y, m, d) = epoch_to_ymd(epoch_secs);
    match lang {
        Lang::Es => format!("{d} {} {y}", MONTHS_ES[(m - 1) as usize]),
        Lang::En => format!("{} {d}, {y}", MONTHS_EN[(m - 1) as usize]),
    }
}

/// Sustituye "{clave}" en una plantilla por los valores dados.
pub fn tf(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in pairs {
        let needle = format!("{{{key}}}");
        out = out.replace(&needle, value);
    }
    out
}

/// Limpia un texto para usarlo como nombre de archivo seguro.
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('_');
        }
    }
    let trimmed: String = out
        .trim_matches(|c| c == '.' || c == '_' || c == '-')
        .to_string();
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed
    }
}

/// Quita sufijos comunes de arquitectura del nombre visible de una AppImage.
pub fn clean_display_name(stem: &str) -> String {
    let suffixes = [
        "-x86_64", "_x86_64", ".x86_64", "-x86-64", "-amd64", "_amd64", "-aarch64", "-arm64",
        "-x64", "-linux", "_linux",
    ];
    let mut name = stem.to_string();
    for suffix in suffixes {
        if let Some(stripped) = name.strip_suffix(suffix) {
            name = stripped.to_string();
            break;
        }
    }
    name.trim_matches(|c| c == '-' || c == '_').to_string()
}

/// Expande "~" al inicio de una ruta.
pub fn expand_tilde(path: &str) -> std::path::PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    std::path::PathBuf::from(path)
}

/// Decodifica %XX en una ruta (los exploradores codifican espacios y acentos
/// al copiar un archivo como URI file://).
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&text[index + 1..index + 3], 16) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// ---------- hora local (sin dependencias: lee /etc/localtime, formato TZif) ----------

fn be_u32(bytes: &[u8], pos: usize) -> Option<u32> {
    if pos + 4 <= bytes.len() {
        Some(u32::from_be_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
        ]))
    } else {
        None
    }
}

fn be_i32(bytes: &[u8], pos: usize) -> Option<i32> {
    be_u32(bytes, pos).map(|v| v as i32)
}

fn be_i64(bytes: &[u8], pos: usize) -> Option<i64> {
    if pos + 8 <= bytes.len() {
        Some(i64::from_be_bytes([
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

#[derive(Clone, Copy)]
struct TzType {
    offset: i32,
    is_dst: bool,
}

/// Datos de una zona TZif: transiciones, el tipo aplicado en cada una y los tipos.
struct TzData {
    transitions: Vec<i64>,
    indices: Vec<u8>,
    types: Vec<TzType>,
}

/// Lee un archivo TZif (v1 o v2/v3) y extrae transiciones, índices y tipos.
fn parse_tzif(bytes: &[u8]) -> Option<TzData> {
    if bytes.len() < 44 || &bytes[0..4] != b"TZif" {
        return None;
    }
    let version = bytes[4];
    let counts_at = |header: usize| -> Option<(usize, usize, usize, usize, usize, usize)> {
        Some((
            be_u32(bytes, header)? as usize,      // isutcnt
            be_u32(bytes, header + 4)? as usize,  // isstdcnt
            be_u32(bytes, header + 8)? as usize,  // leapcnt
            be_u32(bytes, header + 12)? as usize, // timecnt
            be_u32(bytes, header + 16)? as usize, // typecnt
            be_u32(bytes, header + 20)? as usize, // charcnt
        ))
    };
    // Tamaño total del bloque de datos que sigue a un encabezado TZif.
    let block_size = |header: usize, wide: bool| -> Option<usize> {
        let (isut, isstd, leap, timecnt, typecnt, charcnt) = counts_at(header)?;
        Some(
            timecnt * if wide { 9 } else { 5 }
                + typecnt * 6
                + charcnt
                + leap * if wide { 12 } else { 8 }
                + isstd
                + isut,
        )
    };
    let read_block = |header: usize, wide: bool| -> Option<TzData> {
        let (_, _, _, timecnt, typecnt, _) = counts_at(header)?;
        let mut pos = header + 24;
        let mut transitions = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            transitions.push(if wide {
                be_i64(bytes, pos)?
            } else {
                be_i32(bytes, pos)? as i64
            });
            pos += if wide { 8 } else { 4 };
        }
        let mut indices = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            indices.push(*bytes.get(pos)?);
            pos += 1;
        }
        let mut types = Vec::with_capacity(typecnt);
        for _ in 0..typecnt {
            types.push(TzType {
                offset: be_i32(bytes, pos)?,
                is_dst: *bytes.get(pos + 4)? != 0,
            });
            pos += 6;
        }
        Some(TzData {
            transitions,
            indices,
            types,
        })
    };
    if version >= b'2' {
        // El bloque v1 (32 bits) va primero; el bloque real con 64 bits va después.
        let size_v1 = block_size(20, false)?;
        let header2 = 44 + size_v1;
        if bytes.len() < header2 + 44 || &bytes[header2..header2 + 4] != b"TZif" {
            return None;
        }
        read_block(header2 + 20, true)
    } else {
        read_block(20, false)
    }
}

/// Desplazamiento aplicable a un instante: tipo de la última transición <= epoch.
fn offset_at(data: &TzData, epoch: i64) -> i32 {
    if data.types.is_empty() {
        return 0;
    }
    if data.transitions.is_empty() || epoch < data.transitions[0] {
        // Antes de la primera transición: el primer tipo sin horario de verano.
        return data
            .types
            .iter()
            .find(|tt| !tt.is_dst)
            .or_else(|| data.types.first())
            .map(|tt| tt.offset)
            .unwrap_or(0);
    }
    // Última transición <= epoch (búsqueda binaria).
    let mut lo = 0usize;
    let mut hi = data.transitions.len();
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if data.transitions[mid] <= epoch {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let type_index = data.indices.get(lo).copied().unwrap_or(0) as usize;
    data.types.get(type_index).map(|tt| tt.offset).unwrap_or(0)
}

/// Desplazamiento UTC local (segundos) para un instante dado.
/// Fuente: variable TZ (si es un desfase fijo) o /etc/localtime (TZif).
/// Si no se puede determinar, devuelve 0 (UTC).
pub fn local_offset_secs(epoch: i64) -> i32 {
    if let Ok(tz) = std::env::var("TZ") {
        if let Some(offset) = parse_fixed_tz(&tz) {
            return offset;
        }
    }
    if let Ok(bytes) = std::fs::read("/etc/localtime") {
        if let Some(data) = parse_tzif(&bytes) {
            return offset_at(&data, epoch);
        }
    }
    0
}

fn parse_fixed_tz(tz: &str) -> Option<i32> {
    let rest = tz.strip_prefix("UTC").or_else(|| tz.strip_prefix("GMT"))?;
    if rest.is_empty() {
        return Some(0);
    }
    let (sign, rest) = match rest.as_bytes().first()? {
        b'+' => (1, &rest[1..]),
        b'-' => (-1, &rest[1..]),
        _ => return None,
    };
    let mut parts = rest.split(':');
    let hours: i32 = parts.next()?.parse().ok()?;
    let minutes: i32 = parts.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    if hours > 24 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 3600 + minutes * 60))
}

/// Fecha y hora local (desplazamiento aplicado) para un instante epoch.
pub fn local_ymd_hms(epoch: u64) -> (i32, u32, u32, u32, u32, u32) {
    let local = epoch as i64 + local_offset_secs(epoch as i64) as i64;
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (y, m, d) = days_to_ymd(days);
    (
        y,
        m,
        d,
        (secs / 3600) as u32,
        ((secs % 3600) / 60) as u32,
        (secs % 60) as u32,
    )
}

/// Fecha y hora local legible: "2026-11-21 14:03:22".
pub fn fmt_datetime_local(epoch: u64) -> String {
    let (y, m, d, h, mi, s) = local_ymd_hms(epoch);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tamaños_legibles() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(999), "999 B");
        assert_eq!(human_size(1000), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(1_048_576), "1.0 MB");
        assert_eq!(human_size(1_500_000_000), "1.5 GB");
    }

    #[test]
    fn fechas() {
        assert_eq!(epoch_to_ymd(0), (1970, 1, 1));
        assert_eq!(epoch_to_ymd(86_400), (1970, 1, 2));
        assert_eq!(epoch_to_ymd(951_782_400), (2000, 2, 29));
        assert_eq!(fmt_date(0, Lang::Es), "1 ene 1970");
        assert_eq!(fmt_date(0, Lang::En), "Jan 1, 1970");
    }

    #[test]
    fn plantillas() {
        assert_eq!(tf("{n} apps", &[("n", "5")]), "5 apps");
        assert_eq!(tf("sin variables", &[]), "sin variables");
    }

    #[test]
    fn nombres() {
        assert_eq!(clean_display_name("MiApp-1.2-x86_64"), "MiApp-1.2");
        assert_eq!(clean_display_name("Editor_amd64"), "Editor");
        assert_eq!(sanitize_filename("Mi App (2026)!"), "Mi_App_2026");
        assert_eq!(sanitize_filename("///"), "app");
    }

    #[test]
    fn expandir_tilde() {
        assert_eq!(
            expand_tilde("/abs/path"),
            std::path::PathBuf::from("/abs/path")
        );
    }

    #[test]
    fn decodificar_uri() {
        assert_eq!(
            percent_decode("/a b/Mi%20App.AppImage"),
            "/a b/Mi App.AppImage"
        );
        assert_eq!(percent_decode("/a%CC%81rbol"), "/a\u{301}rbol");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn desfase_fijo_de_tz() {
        assert_eq!(parse_fixed_tz("UTC"), Some(0));
        assert_eq!(parse_fixed_tz("UTC+02:00"), Some(7200));
        assert_eq!(parse_fixed_tz("GMT-3"), Some(-10_800));
        assert_eq!(parse_fixed_tz("UTC+02:30"), Some(9000));
        assert_eq!(parse_fixed_tz("Europa/Madrid"), None);
    }

    /// Construye un archivo TZif v1 mínimo para las pruebas.
    fn tzif_v1(transitions: &[i32], indices: &[u8], types: &[(i32, bool)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"TZif");
        bytes.push(0); // versión
        bytes.extend_from_slice(&[0u8; 15]); // reservado
        for value in [0u32, 0, 0, transitions.len() as u32, types.len() as u32, 4] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        for t in transitions {
            bytes.extend_from_slice(&t.to_be_bytes());
        }
        bytes.extend_from_slice(indices);
        for (offset, dst) in types {
            bytes.extend_from_slice(&offset.to_be_bytes());
            bytes.push(*dst as u8);
            bytes.push(0); // abbrind
        }
        bytes.extend_from_slice(b"CET\0"); // charcnt = 4
        bytes
    }

    #[test]
    fn parser_tzif_v1() {
        let data = parse_tzif(&tzif_v1(&[0], &[0], &[(7200, false)])).expect("tzif válido");
        assert_eq!(offset_at(&data, 100), 7200);

        let data = parse_tzif(&tzif_v1(
            &[0, 1_000],
            &[0, 1],
            &[(3_600, false), (7_200, true)],
        ))
        .expect("tzif válido");
        assert_eq!(offset_at(&data, 500), 3_600, "antes del cambio");
        assert_eq!(offset_at(&data, 2_000), 7_200, "tras el cambio (DST)");
        assert_eq!(
            offset_at(&data, -5),
            3_600,
            "antes de la primera transición"
        );

        assert!(parse_tzif(b"no es tzif").is_none());
    }

    #[test]
    fn hora_local_coherente() {
        // Con TZ fija, la hora local debe aplicar el desfase exacto.
        std::env::set_var("TZ", "UTC+02:00");
        let epoch: u64 = 1_700_000_000;
        assert_eq!(local_offset_secs(epoch as i64), 7_200);
        let (y, m, d, h, mi, s) = local_ymd_hms(epoch);
        let utc = epoch + 7_200;
        let days = (utc / 86_400) as i64;
        let rest = utc % 86_400;
        let (ey, em, ed) = days_to_ymd(days);
        assert_eq!(
            (y, m, d, h, mi, s),
            (
                ey,
                em,
                ed,
                (rest / 3600) as u32,
                ((rest % 3600) / 60) as u32,
                (rest % 60) as u32
            )
        );
        std::env::remove_var("TZ");
        assert!(fmt_datetime_local(epoch).contains('-'));
    }
}
