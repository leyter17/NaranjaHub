// NaranjaHub — registro de actividad detallado e informe de fallos.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// Toda acción de la aplicación (integraciones, lanzamientos, ajustes,
// actualizaciones, errores...) se registra aquí con fecha y hora. El registro
// se muestra en la pestaña «Registro», se guarda en disco y puede exportarse
// como informe de fallos y errores en un archivo de texto.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::utils;
use crate::i18n::Strings;

/// Gravedad de un evento del registro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Ok,
    Warn,
    Error,
}

impl Level {
    pub fn key(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Ok => "OK",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    fn from_key(text: &str) -> Option<Level> {
        match text {
            "INFO" => Some(Level::Info),
            "OK" => Some(Level::Ok),
            "WARN" => Some(Level::Warn),
            "ERROR" => Some(Level::Error),
            _ => None,
        }
    }
}

/// Categorías fijas de los eventos (se traducen en la interfaz).
pub const CAT_APP: &str = "app";
pub const CAT_LIBRARY: &str = "biblioteca";
pub const CAT_INTEGRATE: &str = "integrar";
pub const CAT_MENU: &str = "menu";
pub const CAT_TRASH: &str = "papelera";
pub const CAT_SETTINGS: &str = "ajustes";
pub const CAT_UPDATES: &str = "actualizaciones";
pub const CAT_DROP: &str = "arrastrar";
pub const CAT_REPORT: &str = "informe";
pub const CAT_LAUNCH: &str = "lanzamiento";

const MAX_ENTRIES: usize = 600;
const MAX_LOG_BYTES: u64 = 512 * 1024;

#[derive(Clone, Debug)]
pub struct ActivityEntry {
    pub time: u64,
    pub level: Level,
    pub category: &'static str,
    pub message: String,
}

struct Inner {
    entries: Vec<ActivityEntry>,
    file_path: PathBuf,
}

/// Registro de actividad compartible entre hilos (clonable y barato).
#[derive(Clone)]
pub struct ActivityLog {
    inner: Arc<Mutex<Inner>>,
}

impl ActivityLog {
    /// Crea el registro y carga el historial previo desde el archivo.
    pub fn new(file_path: PathBuf) -> Self {
        let log = Self {
            inner: Arc::new(Mutex::new(Inner {
                entries: Vec::new(),
                file_path,
            })),
        };
        log.load_history();
        log
    }

    pub fn file_path(&self) -> PathBuf {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .file_path
            .clone()
    }

    /// Registra un evento: se guarda en memoria y se añade al archivo.
    pub fn log(&self, level: Level, category: &'static str, message: impl Into<String>) {
        let entry = ActivityEntry {
            time: utils::now_secs(),
            level,
            category,
            message: clean_message(&message.into()),
        };
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.entries.push(entry.clone());
        if inner.entries.len() > MAX_ENTRIES {
            let excess = inner.entries.len() - MAX_ENTRIES;
            inner.entries.drain(0..excess);
        }
        append_to_file(&inner.file_path, &format_line(&entry));
    }

    /// Copia de los eventos almacenados (orden cronológico).
    pub fn snapshot(&self) -> Vec<ActivityEntry> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entries
            .clone()
    }

    /// Vacía el registro en memoria y el archivo en disco.
    pub fn clear(&self) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.entries.clear();
        if let Some(parent) = inner.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&inner.file_path, "");
    }

    /// Exporta el informe de fallos y errores como archivo de texto.
    pub fn export_report(
        &self,
        out_dir: &Path,
        t: &Strings,
        context: &ReportContext,
    ) -> Result<PathBuf, String> {
        std::fs::create_dir_all(out_dir)
            .map_err(|e| format!("no se pudo crear {}: {e}", out_dir.display()))?;
        let entries = self.snapshot();
        let warnings = entries.iter().filter(|e| e.level == Level::Warn).count();
        let errors = entries.iter().filter(|e| e.level == Level::Error).count();

        let now = utils::now_secs();
        let stamp = compact_stamp(now);
        let file_path = out_dir.join(format!("informe-naranjahub-{stamp}.txt"));

        let mut text = String::with_capacity(8192);
        let line = "=".repeat(66);
        text.push_str(&line);
        text.push('\n');
        text.push_str(&format!(" {}\n", t.rep_title));
        text.push_str(&format!(" {}\n", t.ab_created_by));
        text.push_str(&line);
        text.push_str("\n\n");

        // --- Resumen ---
        text.push_str(&format!("--- {} ---\n", t.rep_section_summary));
        text.push_str(&format!(
            "  {}\n",
            utils::tf(
                t.rep_count_line,
                &[
                    ("n", entries.len().to_string().as_str()),
                    ("w", warnings.to_string().as_str()),
                    ("e", errors.to_string().as_str()),
                ]
            )
        ));
        text.push('\n');

        // --- Entorno ---
        text.push_str(&format!("--- {} ---\n", t.rep_section_env));
        push_env_line(
            &mut text,
            t.rep_version,
            &format!("NaranjaHub v{}", env!("CARGO_PKG_VERSION")),
        );
        push_env_line(&mut text, t.rep_author, t.ab_created_by);
        push_env_line(&mut text, t.rep_license, "GNU GPL v3");
        push_env_line(&mut text, t.rep_generated, &utils::fmt_datetime_local(now));
        push_env_line(
            &mut text,
            t.rep_os,
            &os_pretty_name().unwrap_or_else(|| t.rep_unknown.to_string()),
        );
        push_env_line(
            &mut text,
            t.rep_desktop,
            &detect_desktop().unwrap_or_else(|| t.rep_unknown.to_string()),
        );
        push_env_line(
            &mut text,
            t.rep_session,
            &detect_session().unwrap_or_else(|| t.rep_unknown.to_string()),
        );
        push_env_line(
            &mut text,
            t.rep_user,
            &detect_user().unwrap_or_else(|| t.rep_unknown.to_string()),
        );
        push_env_line(&mut text, t.rep_cfg_path, &context.config_path);
        push_env_line(&mut text, t.rep_apps_dir, &context.apps_dir);
        push_env_line(
            &mut text,
            t.rep_log_file,
            &self.file_path().display().to_string(),
        );
        text.push('\n');

        // --- Fallos y avisos ---
        text.push_str(&format!("--- {} ---\n", t.rep_section_problems));
        let problems: Vec<&ActivityEntry> = entries
            .iter()
            .filter(|e| e.level == Level::Warn || e.level == Level::Error)
            .collect();
        if problems.is_empty() {
            text.push_str(&format!("  {}\n", t.rep_no_problems));
        } else {
            for entry in problems {
                text.push_str(&format!("  {}\n", format_entry_line(entry, t)));
            }
        }
        text.push('\n');

        // --- Registro completo ---
        text.push_str(&format!("--- {} ---\n", t.rep_section_full));
        if entries.is_empty() {
            text.push_str(&format!("  {}\n", t.rep_no_problems));
        } else {
            for entry in &entries {
                text.push_str(&format!("  {}\n", format_entry_line(entry, t)));
            }
        }
        text.push('\n');

        std::fs::write(&file_path, text)
            .map_err(|e| format!("no se pudo escribir el informe: {e}"))?;
        Ok(file_path)
    }

    fn load_history(&self) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Ok(text) = std::fs::read_to_string(&inner.file_path) else {
            return;
        };
        let lines: Vec<&str> = text.lines().collect();
        for line in lines.iter().rev().take(MAX_ENTRIES).rev() {
            let mut parts = line.splitn(4, '\t');
            let (Some(time), Some(level), Some(category), Some(message)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let Ok(time) = time.parse::<u64>() else {
                continue;
            };
            let Some(level) = Level::from_key(level) else {
                continue;
            };
            let Some(category) = valid_category(category) else {
                continue;
            };
            if message.is_empty() {
                continue;
            }
            inner.entries.push(ActivityEntry {
                time,
                level,
                category,
                message: message.to_string(),
            });
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReportContext {
    pub config_path: String,
    pub apps_dir: String,
}

/// Escritorio detectado (XDG_CURRENT_DESKTOP o DESKTOP_SESSION).
pub fn detect_desktop() -> Option<String> {
    for key in ["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Tipo de sesión gráfica (x11 o wayland).
pub fn detect_session() -> Option<String> {
    for key in ["XDG_SESSION_TYPE", "WAYLAND_DISPLAY", "DISPLAY"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(if key == "DISPLAY" {
                    "x11".to_string()
                } else {
                    value
                });
            }
        }
    }
    None
}

fn detect_user() -> Option<String> {
    for key in ["USER", "LOGNAME"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn os_pretty_name() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string())
}

fn clean_message(message: &str) -> String {
    message.replace(['\t', '\n', '\r'], " ")
}

fn format_line(entry: &ActivityEntry) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        entry.time,
        entry.level.key(),
        entry.category,
        clean_message(&entry.message)
    )
}

fn valid_category(category: &str) -> Option<&'static str> {
    match category {
        CAT_APP => Some(CAT_APP),
        CAT_LIBRARY => Some(CAT_LIBRARY),
        CAT_INTEGRATE => Some(CAT_INTEGRATE),
        CAT_MENU => Some(CAT_MENU),
        CAT_TRASH => Some(CAT_TRASH),
        CAT_SETTINGS => Some(CAT_SETTINGS),
        CAT_UPDATES => Some(CAT_UPDATES),
        CAT_DROP => Some(CAT_DROP),
        CAT_REPORT => Some(CAT_REPORT),
        CAT_LAUNCH => Some(CAT_LAUNCH),
        _ => None,
    }
}

fn append_to_file(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_LOG_BYTES {
            let old = path.with_extension("log.old");
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::rename(path, &old);
        }
    }
    use std::io::Write;
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

fn push_env_line(text: &mut String, label: &str, value: &str) {
    text.push_str(&format!("  {label:<26} {value}\n"));
}

fn format_entry_line(entry: &ActivityEntry, _t: &Strings) -> String {
    // Las etiquetas de nivel se mantienen en clave fija (INFO/OK/WARN/ERROR)
    // para que el informe sea fácil de filtrar en cualquier idioma.
    format!(
        "[{}] [{}] [{}] {}",
        utils::fmt_datetime_local(entry.time),
        entry.level.key(),
        entry.category,
        entry.message
    )
}

/// "20261121-140322" a partir de la hora local.
fn compact_stamp(epoch: u64) -> String {
    let (y, m, d, h, mi, s) = utils::local_ymd_hms(epoch);
    format!("{y:04}{m:02}{d:02}-{h:02}{mi:02}{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Lang, ES};

    #[test]
    fn niveles_y_formato_de_linea() {
        assert_eq!(Level::Error.key(), "ERROR");
        assert_eq!(Level::from_key("WARN"), Some(Level::Warn));
        assert_eq!(Level::from_key("otro"), None);
        let entry = ActivityEntry {
            time: 1_000,
            level: Level::Error,
            category: CAT_INTEGRATE,
            message: "fallo\tcon\tsaltos\n".to_string(),
        };
        let line = format_line(&entry);
        assert_eq!(line, "1000\tERROR\tintegrar\tfallo con saltos ");
    }

    #[test]
    fn historial_y_limpiar() {
        let base = std::env::temp_dir().join(format!("nh-test-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let path = base.join("actividad.log");
        let log = ActivityLog::new(path.clone());
        log.log(Level::Info, CAT_APP, "uno");
        log.log(Level::Error, CAT_INTEGRATE, "dos");
        assert_eq!(log.snapshot().len(), 2);
        // El archivo existe y se puede volver a cargar en un registro nuevo.
        let log2 = ActivityLog::new(path.clone());
        assert_eq!(log2.snapshot().len(), 2);
        assert_eq!(log2.snapshot()[0].message, "uno");
        assert_eq!(log2.snapshot()[1].level, Level::Error);
        log2.clear();
        assert_eq!(log2.snapshot().len(), 0);
        assert_eq!(
            ActivityLog::new(path).snapshot().len(),
            0,
            "tras limpiar el archivo queda vacío"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn exportacion_de_informe() {
        let base = std::env::temp_dir().join(format!("nh-test-rep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let log = ActivityLog::new(base.join("actividad.log"));
        log.log(Level::Ok, CAT_INTEGRATE, "integración correcta");
        log.log(Level::Warn, CAT_UPDATES, "sin información de actualización");
        log.log(Level::Error, CAT_LAUNCH, "no se pudo lanzar");
        let out_dir = base.join("informes");
        let context = ReportContext {
            config_path: "/tmp/config.json".to_string(),
            apps_dir: "/tmp/apps".to_string(),
        };
        let report = log
            .export_report(&out_dir, Lang::Es.strings(), &context)
            .expect("informe");
        assert!(report.exists());
        let text = std::fs::read_to_string(&report).unwrap();
        assert!(text.contains("[ERROR]"));
        assert!(text.contains("[WARN]"));
        assert!(text.contains("[OK]"));
        assert!(text.contains("no se pudo lanzar"));
        assert!(text.contains("GNU GPL v3"));
        assert!(text.contains("Leyter Vázquez. V"));
        assert!(report
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("informe-naranjahub-"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn deteccion_de_entorno_no_panic() {
        let _ = detect_desktop();
        let _ = detect_session();
        let _ = os_pretty_name();
        assert_eq!(Lang::Es.strings().rep_title, ES.rep_title);
    }
}
