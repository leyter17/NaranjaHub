// NaranjaHub — estado y ciclo de vida de la aplicación.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32};

use crate::browser::Browser;
use crate::config::{self, Config};
use crate::core::activity::{
    ActivityLog, Level, CAT_APP, CAT_DROP, CAT_INTEGRATE, CAT_LAUNCH, CAT_LIBRARY, CAT_MENU,
    CAT_SETTINGS, CAT_TRASH, CAT_UPDATES,
};
use crate::core::discovery::{self, AppImageEntry};
use crate::core::integration::{self, DirPaths};
use crate::core::trash;
use crate::core::updater::{self, GhUpdateInfo};
use crate::core::updstore;
use crate::core::utils;
use crate::i18n::Strings;
use crate::ui_common::{Toast, ToastKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Library,
    Integrate,
    Updates,
    Activity,
    Settings,
    About,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Name,
    Date,
    Size,
}

/// Filtro de la pestaña Registro.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActivityFilter {
    All,
    Problems,
}

/// Tiempo máximo TOTAL de una comprobación de actualización antes de darla
/// por fallida. La petición a la API de GitHub es pequeña; si tarda más es
/// que no hay conexión (DNS colgado, red caída, cortafuegos...). Sin este
/// límite la ficha se quedaba eternamente en «Comprobando...» sin red.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(45);

/// Tiempo máximo SIN ningún cambio de progreso en trabajos de copia o
/// descarga: el progreso avanza constantemente, así que un bloqueo largo
/// indica una operación muerta (disco o red desconectados).
pub const STALL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Working(String),
    Checking,
    UpToDate,
    Available {
        asset: String,
        url: String,
        size: u64,
    },
    Progress {
        label: String,
        got: u64,
        total: Option<u64>,
    },
    Done(String),
    Failed(String),
}

#[derive(Clone, Copy, PartialEq)]
pub enum JobKind {
    Integrate,
    Analyze,
    Update,
}

pub struct Job {
    pub kind: JobKind,
    pub status: Arc<Mutex<JobStatus>>,
    pub toasted_done: bool,
    pub toasted_available: bool,
    pub silent: bool,
    /// Instante en que se lanzó el trabajo (límite total del vigilante).
    pub started: Instant,
    /// Último cambio de estado visto (límite de estancamiento).
    pub last_change: Instant,
    /// Último estado visto por la interfaz.
    pub last_seen: JobStatus,
}

/// Vigilante de trabajos: decide si un trabajo lleva demasiado tiempo y debe
/// abortarse con un error claro en vez de quedarse colgado para siempre.
/// Devuelve el mensaje de error si hay que abortarlo, o None si sigue.
///
/// - Comprobaciones (JobKind::Update en estado Checking): límite TOTAL.
/// - Copias y descargas (estado Progress) y análisis: límite de
///   ESTANCAMIENTO (sin cambios de progreso durante STALL_TIMEOUT).
/// - Estados terminados (Al día, Error, Nueva versión...) nunca caducan.
pub fn job_timeout_error(
    kind: JobKind,
    status: &JobStatus,
    total_elapsed: Duration,
    stall: Duration,
) -> Option<String> {
    let in_flight = matches!(
        status,
        JobStatus::Working(_) | JobStatus::Checking | JobStatus::Progress { .. }
    );
    if !in_flight {
        return None;
    }
    if kind == JobKind::Update && matches!(status, JobStatus::Checking) {
        if total_elapsed >= CHECK_TIMEOUT {
            return Some(
                "tiempo agotado al comprobar (¿hay conexión a internet?)".to_string(),
            );
        }
        return None;
    }
    if stall >= STALL_TIMEOUT {
        return Some(
            "la operación lleva demasiado tiempo sin avanzar y se ha detenido".to_string(),
        );
    }
    None
}

impl Job {
    pub fn status(&self) -> JobStatus {
        self.status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

pub struct NaranjaHubApp {
    pub tab: Tab,
    pub config: Config,
    pub config_path: PathBuf,
    pub dirs: DirPaths,
    pub library: Vec<AppImageEntry>,
    pub search: String,
    pub sort: Sort,
    pub logo: Option<egui::TextureHandle>,
    pub generic_icon: Option<egui::TextureHandle>,
    /// Iconos reales de las AppImages integradas (ruta del PNG → mtime, textura).
    pub icon_cache: HashMap<PathBuf, (u64, egui::TextureHandle)>,
    pub integrate_browser: Browser,
    pub show_browser: bool,
    pub create_entry: bool,
    pub move_original: bool,
    /// Ruta pegada a mano en la pestaña Integrar (funciona en Wayland también).
    pub paste_path: String,
    /// Backend gráfico en uso: "x11" o "wayland" (para avisos y registro).
    pub backend: String,
    pub settings_browser: Option<Browser>,
    pub jobs: HashMap<PathBuf, Job>,
    pub toasts: Vec<Toast>,
    pub pending_delete: Option<PathBuf>,
    pub activity: ActivityLog,
    pub activity_filter: ActivityFilter,
    pub report_browser: Option<Browser>,
    /// Agente para DESCARGAS: sin límite global (los archivos pueden ser
    /// grandes) pero con límites por lectura.
    pub agent: ureq::Agent,
    /// Agente para COMPROBACIONES: límite global de 30 s para que la ficha
    /// nunca se quede en «Comprobando...» sin conexión.
    pub check_agent: ureq::Agent,
    /// Direcciones de actualización definidas a mano por el usuario
    /// (ruta de la AppImage → cadena gh-releases-zsync). Tienen prioridad
    /// sobre la información embebida en el archivo.
    pub upd_overrides: updstore::UpdateOverrides,
    pub overrides_path: PathBuf,
    /// Editor de la dirección de actualización: (ruta, texto en edición).
    pub upd_edit: Option<(PathBuf, String)>,
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img
        .into_raw()
        .chunks_exact(4)
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    Some(ctx.load_texture(
        name,
        egui::ColorImage { size, pixels },
        egui::TextureOptions::LINEAR,
    ))
}

impl NaranjaHubApp {
    pub fn new(cc: &eframe::CreationContext<'_>, backend: &str) -> Self {
        let home = dirs::home_dir();
        let config_dir = dirs::config_dir()
            .map(|d| d.join("NaranjaHub"))
            .unwrap_or_else(|| PathBuf::from(".naranjahub"));
        let data_dir = dirs::data_dir()
            .map(|d| d.join("NaranjaHub"))
            .unwrap_or_else(|| PathBuf::from(".naranjahub-data"));
        let config_path = config_dir.join("config.json");
        let had_config = config_path.exists();
        let config = config::load(&config_path);
        let desktops_dir = home
            .map(|h| h.join(".local/share/applications"))
            .unwrap_or_else(|| PathBuf::from(".local/share/applications"));
        let hicolor_dir = dirs::data_dir()
            .map(|d| d.join("icons/hicolor"))
            .unwrap_or_else(|| PathBuf::from(".local/share/icons/hicolor"));
        let dirs = DirPaths {
            apps_dir: utils::expand_tilde(&config.apps_dir),
            icons_dir: data_dir.join("icons"),
            hicolor_dir,
            desktops_dir,
        };
        let _ = std::fs::create_dir_all(&dirs.apps_dir);
        let _ = std::fs::create_dir_all(&dirs.icons_dir);
        let _ = std::fs::create_dir_all(&dirs.desktops_dir);
        let activity = ActivityLog::new(data_dir.join("logs").join("actividad.log"));
        let overrides_path = data_dir.join("updates.json");
        let mut app = Self {
            tab: Tab::Library,
            config,
            config_path,
            dirs,
            library: Vec::new(),
            search: String::new(),
            sort: Sort::Name,
            logo: load_texture(
                &cc.egui_ctx,
                "naranjahub-logo",
                include_bytes!("../assets/icon-512.png"),
            ),
            generic_icon: load_texture(
                &cc.egui_ctx,
                "naranjahub-generic",
                include_bytes!("../assets/generic-app.png"),
            ),
            icon_cache: HashMap::new(),
            integrate_browser: Browser::at_home(),
            show_browser: false,
            create_entry: true,
            move_original: false,
            paste_path: String::new(),
            backend: backend.to_string(),
            settings_browser: None,
            jobs: HashMap::new(),
            toasts: Vec::new(),
            pending_delete: None,
            activity,
            activity_filter: crate::app::ActivityFilter::All,
            report_browser: None,
            agent: updater::new_agent(),
            check_agent: updater::new_check_agent(),
            upd_overrides: updstore::load(&overrides_path),
            overrides_path,
            upd_edit: None,
        };
        app.reload_library();
        {
            let t = app.t();
            app.log_ok(
                CAT_APP,
                utils::tf(t.log_started, &[("v", env!("CARGO_PKG_VERSION"))]),
            );
            app.log_info(
                CAT_APP,
                utils::tf(
                    t.log_env,
                    &[
                        (
                            "desktop",
                            crate::core::activity::detect_desktop()
                                .unwrap_or_else(|| t.rep_unknown.to_string())
                                .as_str(),
                        ),
                        (
                            "session",
                            crate::core::activity::detect_session()
                                .unwrap_or_else(|| t.rep_unknown.to_string())
                                .as_str(),
                        ),
                    ],
                ),
            );
            app.log_info(
                CAT_APP,
                utils::tf(
                    t.log_backend,
                    &[
                        ("backend", backend),
                        (
                            "session",
                            crate::core::activity::detect_session()
                                .unwrap_or_else(|| t.rep_unknown.to_string())
                                .as_str(),
                        ),
                    ],
                ),
            );
            if had_config {
                app.log_info(
                    CAT_SETTINGS,
                    utils::tf(
                        t.log_config_loaded,
                        &[("path", app.config_path.display().to_string().as_str())],
                    ),
                );
            } else {
                app.log_info(CAT_SETTINGS, t.log_config_missing);
            }
            app.log_info(
                CAT_LIBRARY,
                utils::tf(
                    t.log_library,
                    &[
                        ("dir", app.dirs.apps_dir.display().to_string().as_str()),
                        ("n", app.library.len().to_string().as_str()),
                    ],
                ),
            );
            if !app.upd_overrides.is_empty() {
                app.log_info(
                    CAT_UPDATES,
                    utils::tf(
                        t.log_overrides_loaded,
                        &[("n", app.upd_overrides.len().to_string().as_str())],
                    ),
                );
            }
            if app.config.check_updates_on_start {
                app.log_info(CAT_UPDATES, t.log_autoupdate);
            }
        }
        if app.config.check_updates_on_start {
            app.check_all(true);
        }
        // Gancho opcional para abrir directamente una pestaña (p. ej. para capturas):
        // NARANJAHUB_TAB=integrate|updates|activity|settings|about
        if let Ok(tab) = std::env::var("NARANJAHUB_TAB") {
            match tab.as_str() {
                "integrate" => app.tab = Tab::Integrate,
                "updates" => app.tab = Tab::Updates,
                "activity" => app.tab = Tab::Activity,
                "settings" => app.tab = Tab::Settings,
                "about" => app.tab = Tab::About,
                _ => {}
            }
        }
        // Ganchos opcionales para capturas: explorador abierto y archivo preseleccionado.
        if std::env::var("NARANJAHUB_BROWSER").as_deref() == Ok("1") {
            app.show_browser = true;
        }
        if let Ok(path) = std::env::var("NARANJAHUB_SELECT") {
            let path = PathBuf::from(path);
            if path.is_file() {
                if let Some(parent) = path.parent().map(|p| p.to_path_buf()) {
                    app.integrate_browser = Browser {
                        current: parent,
                        entries: Vec::new(),
                        selected: Some(path),
                        error: None,
                    };
                    app.integrate_browser.refresh();
                }
                app.show_browser = true;
            }
        }
        app
    }

    pub fn t(&self) -> &'static Strings {
        self.config.lang.strings()
    }

    pub fn toast(&mut self, kind: ToastKind, msg: impl Into<String>) {
        self.toasts.push(Toast {
            kind,
            msg: msg.into(),
            born: std::time::Instant::now(),
        });
    }

    pub fn toast_ok(&mut self, msg: impl Into<String>) {
        self.toast(ToastKind::Ok, msg);
    }

    pub fn toast_err(&mut self, msg: impl Into<String>) {
        self.toast(ToastKind::Error, msg);
    }

    pub fn toast_info(&mut self, msg: impl Into<String>) {
        self.toast(ToastKind::Info, msg);
    }

    // ---------- registro de actividad ----------

    pub fn log_info(&self, category: &'static str, msg: impl Into<String>) {
        self.activity.log(Level::Info, category, msg);
    }

    pub fn log_ok(&self, category: &'static str, msg: impl Into<String>) {
        self.activity.log(Level::Ok, category, msg);
    }

    pub fn log_warn(&self, category: &'static str, msg: impl Into<String>) {
        self.activity.log(Level::Warn, category, msg);
    }

    pub fn log_err(&self, category: &'static str, msg: impl Into<String>) {
        self.activity.log(Level::Error, category, msg);
    }

    pub fn save_config(&self) -> Result<(), String> {
        config::save(&self.config_path, &self.config)
    }

    /// Icono real de una AppImage integrada (la copia PNG que NaranjaHub
    /// instaló al integrar). Con caché por ruta y mtime, y auto-recarga:
    /// si todavía no hay icono devuelve None y la biblioteca muestra el
    /// genérico hasta que aparezca el archivo.
    pub fn icon_for(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        let slug = integration::slug_for(path);
        let file = self.dirs.icons_dir.join(format!("{slug}.png"));
        let mtime = std::fs::metadata(&file)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let Some(mtime) = mtime else {
            self.icon_cache.remove(&file);
            return None;
        };
        if let Some((cached_time, tex)) = self.icon_cache.get(&file) {
            if *cached_time == mtime {
                return Some(tex.clone());
            }
        }
        let bytes = std::fs::read(&file).ok()?;
        let tex = load_texture(ctx, &format!("naranjahub-icono-{slug}"), &bytes)?;
        self.icon_cache.insert(file, (mtime, tex.clone()));
        Some(tex)
    }

    pub fn reload_library(&mut self) {
        let (entries_map, infos) = discovery::scan_our_desktop_entries(&self.dirs.desktops_dir);
        let mut list = discovery::scan_apps_dir(&self.dirs.apps_dir);
        for entry in &mut list {
            if let Some(desktop) = entries_map.get(&entry.path) {
                entry.desktop_path = Some(desktop.clone());
                if let Ok(text) = std::fs::read_to_string(desktop) {
                    let keys = integration::parse_desktop_file(&text);
                    entry.original_name = keys.get("X-NaranjaHub-OriginalName").cloned();
                    entry.version = keys.get("X-NaranjaHub-Version").cloned();
                }
            }
            entry.update_info = match self.upd_overrides.get(&entry.path.display().to_string()) {
                // La dirección definida a mano por el usuario tiene prioridad
                // sobre la información embebida en la AppImage.
                Some(raw) => updater::parse_update_information(raw),
                None => infos.get(&entry.path).cloned(),
            };
        }
        match self.sort {
            Sort::Name => list.sort_by_key(|a| a.name.to_lowercase()),
            Sort::Date => list.sort_by_key(|a| std::cmp::Reverse(a.mtime)),
            Sort::Size => list.sort_by_key(|a| std::cmp::Reverse(a.size)),
        }
        self.library = list;
    }

    // ---------- acciones ----------

    pub fn launch(&mut self, path: &Path) {
        let t = self.t();
        if !path.exists() {
            self.toast_err(format!("no existe: {}", path.display()));
            self.log_err(
                CAT_LAUNCH,
                utils::tf(
                    t.log_missing_err,
                    &[("path", path.display().to_string().as_str())],
                ),
            );
            return;
        }
        if let Err(err) = integration::set_executable(path) {
            self.toast_err(err.clone());
            self.log_err(
                CAT_LAUNCH,
                utils::tf(
                    t.log_noexec_err,
                    &[
                        ("path", path.display().to_string().as_str()),
                        ("err", err.as_str()),
                    ],
                ),
            );
            return;
        }
        let result = Command::new(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match result {
            Ok(_) => {
                self.toast_ok(t.toast_launch_ok);
                self.log_ok(
                    CAT_LAUNCH,
                    utils::tf(
                        t.log_launched,
                        &[("path", path.display().to_string().as_str())],
                    ),
                );
            }
            Err(err) => {
                self.toast_err(format!("no se pudo lanzar: {err}"));
                self.log_err(
                    CAT_LAUNCH,
                    utils::tf(
                        t.log_launch_err,
                        &[
                            ("path", path.display().to_string().as_str()),
                            ("err", err.to_string().as_str()),
                        ],
                    ),
                );
            }
        }
    }

    pub fn open_in_file_manager(&mut self, path: &Path) {
        let t = self.t();
        let result = Command::new("xdg-open")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match result {
            Ok(_) => self.log_info(
                CAT_APP,
                utils::tf(
                    t.log_opening,
                    &[("path", path.display().to_string().as_str())],
                ),
            ),
            Err(err) => {
                self.toast_err(format!("{}: {err}", t.toast_err_open));
                self.log_err(
                    CAT_APP,
                    utils::tf(
                        t.log_open_err,
                        &[
                            ("path", path.display().to_string().as_str()),
                            ("err", err.to_string().as_str()),
                        ],
                    ),
                );
            }
        }
    }

    pub fn integrate_path(&mut self, src: PathBuf) {
        let dirs = self.dirs.clone();
        let options = integration::IntegrateOptions {
            create_entry: self.create_entry,
            move_original: self.move_original,
        };
        {
            let t = self.t();
            self.log_info(
                CAT_INTEGRATE,
                utils::tf(
                    t.log_integrating,
                    &[
                        ("path", src.display().to_string().as_str()),
                        (
                            "entry",
                            if options.create_entry {
                                t.log_yes
                            } else {
                                t.log_no
                            },
                        ),
                        (
                            "move",
                            if options.move_original {
                                t.log_yes
                            } else {
                                t.log_no
                            },
                        ),
                    ],
                ),
            );
        }
        let initial = JobStatus::Working(self.t().int_copying.to_string());
        let status = Arc::new(Mutex::new(initial.clone()));
        let status_thread = status.clone();
        let src_thread = src.clone();
        let log = self.activity.clone();
        let t_static = self.t();
        let options_thread = options.clone();
        let dirs_thread = dirs.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            // El cuerpo va dentro de catch_unwind: si algo entra en pánico el
            // estado NO puede quedarse congelado en «Trabajando...».
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let progress = |got: u64, total: Option<u64>| {
                    let mut guard = status_thread
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *guard = JobStatus::Progress {
                        label: String::new(),
                        got,
                        total,
                    };
                };
                integration::integrate(&src_thread, &options_thread, &dirs_thread, &progress)
            }));
            let elapsed = started.elapsed().as_millis();
            let new_status = match outcome {
                Ok(Ok(outcome)) => {
                    let entry_info = outcome
                        .desktop_path
                        .as_ref()
                        .map(|d| d.display().to_string())
                        .unwrap_or_else(|| t_static.log_no_entry.to_string());
                    log.log(
                        Level::Ok,
                        CAT_INTEGRATE,
                        utils::tf(
                            t_static.log_integrated,
                            &[
                                ("name", outcome.app_name.as_str()),
                                ("ms", elapsed.to_string().as_str()),
                                ("dest", outcome.new_path.display().to_string().as_str()),
                                ("entry", entry_info.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Done(outcome.app_name)
                }
                Ok(Err(err)) => {
                    log.log(
                        Level::Error,
                        CAT_INTEGRATE,
                        utils::tf(
                            t_static.log_integrate_err,
                            &[
                                ("path", src_thread.display().to_string().as_str()),
                                ("err", err.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Failed(err)
                }
                Err(_) => {
                    let err = "error interno inesperado al integrar".to_string();
                    log.log(
                        Level::Error,
                        CAT_INTEGRATE,
                        utils::tf(
                            t_static.log_integrate_err,
                            &[
                                ("path", src_thread.display().to_string().as_str()),
                                ("err", err.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Failed(err)
                }
            };
            let mut guard = status_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = new_status;
        });
        self.jobs.insert(
            src,
            Job {
                kind: JobKind::Integrate,
                status,
                toasted_done: false,
                toasted_available: false,
                silent: false,
                started: Instant::now(),
                last_change: Instant::now(),
                last_seen: initial,
            },
        );
    }

    pub fn check_updates_for(
        &mut self,
        path: PathBuf,
        info: GhUpdateInfo,
        compare_name: String,
        current_version: Option<String>,
        silent: bool,
    ) {
        let initial = JobStatus::Checking;
        let status = Arc::new(Mutex::new(initial.clone()));
        let status_thread = status.clone();
        // Agente con límite global de 30 s: sin conexión la petición caduca
        // sola y no se queda colgada (además del vigilante de la interfaz).
        let agent = self.check_agent.clone();
        let log = self.activity.clone();
        let t_static = self.t();
        let path_thread = path.clone();
        std::thread::spawn(move || {
            // catch_unwind: un pánico inesperado deja Failed en vez de un
            // estado «Comprobando...» congelado para siempre.
            let outcome =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    updater::check_latest(&agent, &info)
                }));
            let new_status = match outcome {
                // «Al día»: mismo nombre de archivo o, si hay versiones
                // comparables, ninguna versión publicada superior a la local.
                Ok(Ok(asset))
                    if !updater::is_newer_asset(
                        &asset.name,
                        current_version.as_deref(),
                        &compare_name,
                    ) =>
                {
                    log.log(
                        Level::Ok,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_uptodate,
                            &[("path", path_thread.display().to_string().as_str())],
                        ),
                    );
                    JobStatus::UpToDate
                }
                Ok(Ok(asset)) => {
                    log.log(
                        Level::Info,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_available,
                            &[
                                ("path", path_thread.display().to_string().as_str()),
                                ("asset", asset.name.as_str()),
                                ("size", utils::human_size(asset.size).as_str()),
                            ],
                        ),
                    );
                    JobStatus::Available {
                        asset: asset.name,
                        url: asset.url,
                        size: asset.size,
                    }
                }
                Ok(Err(err)) => {
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_check_err,
                            &[
                                ("path", path_thread.display().to_string().as_str()),
                                ("err", err.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Failed(err)
                }
                Err(_) => {
                    let err = "error interno inesperado al comprobar".to_string();
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_check_err,
                            &[
                                ("path", path_thread.display().to_string().as_str()),
                                ("err", err.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Failed(err)
                }
            };
            let mut guard = status_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = new_status;
        });
        self.jobs.insert(
            path,
            Job {
                kind: JobKind::Update,
                status,
                toasted_done: false,
                toasted_available: false,
                silent,
                started: Instant::now(),
                last_change: Instant::now(),
                last_seen: initial,
            },
        );
    }

    pub fn check_all(&mut self, silent: bool) {
        if self.library.is_empty() {
            self.toast_info(self.t().err_no_info);
            return;
        }
        // Con información declarada → comprobar ahora. Sin información →
        // analizar primero (lee la sección .upd_info del ELF y la guarda en el
        // acceso directo) y, al terminar, la comprobación se lanza sola.
        let mut to_check: Vec<(PathBuf, GhUpdateInfo, String, Option<String>)> = Vec::new();
        let mut to_analyze: Vec<PathBuf> = Vec::new();
        for entry in &self.library {
            match entry.update_info.clone() {
                Some(info) => {
                    let compare = entry
                        .original_name
                        .clone()
                        .unwrap_or_else(|| file_name_string(&entry.path));
                    to_check.push((entry.path.clone(), info, compare, entry.version.clone()));
                }
                None => to_analyze.push(entry.path.clone()),
            }
        }
        if !to_check.is_empty() {
            self.log_info(
                CAT_UPDATES,
                utils::tf(
                    self.t().log_checking,
                    &[("n", to_check.len().to_string().as_str())],
                ),
            );
        }
        if !to_analyze.is_empty() && !silent {
            self.log_info(
                CAT_UPDATES,
                utils::tf(
                    self.t().log_analyzing_batch,
                    &[("n", to_analyze.len().to_string().as_str())],
                ),
            );
        }
        for (path, info, compare, version) in to_check {
            self.check_updates_for(path, info, compare, version, silent);
        }
        if !silent {
            for path in to_analyze {
                self.analyze_path(path);
            }
        }
    }

    pub fn analyze_path(&mut self, path: PathBuf) {
        let dirs = self.dirs.clone();
        let initial = JobStatus::Working(self.t().upd_working.to_string());
        let status = Arc::new(Mutex::new(initial.clone()));
        let status_thread = status.clone();
        let path_thread = path.clone();
        let log = self.activity.clone();
        let t_static = self.t();
        let dirs_thread = dirs.clone();
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                integration::refresh_integration(&path_thread, &dirs_thread)
            }));
            let new_status = match outcome {
                Ok(Ok(())) => {
                    log.log(
                        Level::Ok,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_analyzed,
                            &[("path", path_thread.display().to_string().as_str())],
                        ),
                    );
                    JobStatus::Done("ok".to_string())
                }
                Ok(Err(err)) => {
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_analyze_err,
                            &[
                                ("path", path_thread.display().to_string().as_str()),
                                ("err", err.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Failed(err)
                }
                Err(_) => {
                    let err = "error interno inesperado al analizar".to_string();
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_analyze_err,
                            &[
                                ("path", path_thread.display().to_string().as_str()),
                                ("err", err.as_str()),
                            ],
                        ),
                    );
                    JobStatus::Failed(err)
                }
            };
            let mut guard = status_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = new_status;
        });
        self.jobs.insert(
            path,
            Job {
                kind: JobKind::Analyze,
                status,
                toasted_done: false,
                toasted_available: false,
                silent: false,
                started: Instant::now(),
                last_change: Instant::now(),
                last_seen: initial,
            },
        );
    }

    pub fn download_update(&mut self, path: PathBuf, url: String, asset_name: String, size: u64) {
        let dirs = self.dirs.clone();
        let agent = self.agent.clone();
        let log = self.activity.clone();
        let t_static = self.t();
        let initial = JobStatus::Progress {
            label: asset_name.clone(),
            got: 0,
            total: Some(size),
        };
        let status = Arc::new(Mutex::new(initial.clone()));
        self.log_info(
            CAT_UPDATES,
            utils::tf(
                self.t().log_downloading,
                &[
                    ("asset", asset_name.as_str()),
                    ("size", utils::human_size(size).as_str()),
                    ("path", path.display().to_string().as_str()),
                ],
            ),
        );
        let status_thread = status.clone();
        let path_thread = path.clone();
        let dirs_thread = dirs.clone();
        std::thread::spawn(move || {
            // El cuerpo entero va en catch_unwind: si algo entra en pánico el
            // estado queda Failed y no congelado en la descarga.
            // Se distingue el paso que falló (descarga o instalación) para
            // registrar el error con el texto correcto.
            enum StepError {
                Download(String),
                Install(String),
            }
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // Sufijo .part para que la descarga NO aparezca en la biblioteca
                // mientras está a medias (la biblioteca solo lista *.AppImage).
                let tmp = dirs_thread
                    .apps_dir
                    .join(format!(".naranjahub-descarga-{}.part", asset_name));
                let progress = |got: u64, total: Option<u64>| {
                    let mut guard = status_thread
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *guard = JobStatus::Progress {
                        label: String::new(),
                        got,
                        total,
                    };
                };
                let downloaded = updater::download(&agent, &url, &tmp, &progress);
                if let Err(err) = downloaded {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(StepError::Download(err));
                }
                // Instalación con respaldo: si el asset tiene el mismo nombre se
                // sustituye en sitio; si no, se añade nuevo y se limpia el viejo.
                match integration::apply_update(&path_thread, &tmp, &asset_name, &dirs_thread) {
                    Ok(dest) => Ok(dest),
                    Err(err) => Err(StepError::Install(err)),
                }
            }));
            let new_status = match outcome {
                Ok(Ok(dest)) => {
                    log.log(
                        Level::Ok,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_installed,
                            &[
                                ("asset", asset_name.as_str()),
                                ("dest", dest.display().to_string().as_str()),
                            ],
                        ),
                    );
                    JobStatus::Done(dest.display().to_string())
                }
                Ok(Err(StepError::Download(err))) => {
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_download_err,
                            &[("asset", asset_name.as_str()), ("err", err.as_str())],
                        ),
                    );
                    JobStatus::Failed(err)
                }
                Ok(Err(StepError::Install(err))) => {
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_install_err,
                            &[("asset", asset_name.as_str()), ("err", err.as_str())],
                        ),
                    );
                    JobStatus::Failed(err)
                }
                Err(_) => {
                    let err = "error interno inesperado al actualizar".to_string();
                    log.log(
                        Level::Error,
                        CAT_UPDATES,
                        utils::tf(
                            t_static.log_install_err,
                            &[("asset", asset_name.as_str()), ("err", err.as_str())],
                        ),
                    );
                    JobStatus::Failed(err)
                }
            };
            let mut guard = status_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = new_status;
        });
        self.jobs.insert(
            path,
            Job {
                kind: JobKind::Update,
                status,
                toasted_done: false,
                toasted_available: false,
                silent: false,
                started: Instant::now(),
                last_change: Instant::now(),
                last_seen: initial,
            },
        );
    }

    // ---------- direcciones de actualización manuales ----------

    /// Guarda (o sustituye) la dirección de actualización manual de una
    /// AppImage y, si es válida, lanza la comprobación de inmediato.
    pub fn set_update_address(&mut self, path: PathBuf, raw: &str) {
        let t = self.t();
        let local_name = file_name_string(&path);
        match updater::parse_manual_address(raw, &local_name) {
            Ok(info) => {
                let canonical = updater::canonical_update_information(&info);
                self.upd_overrides
                    .insert(path.display().to_string(), canonical.clone());
                let save_result = updstore::save(&self.overrides_path, &self.upd_overrides);
                let name = discovery::display_name(&path);
                match save_result {
                    Ok(()) => {
                        self.toast_ok(utils::tf(t.upd_addr_saved, &[("name", name.as_str())]));
                        self.log_ok(
                            CAT_UPDATES,
                            utils::tf(
                                t.log_addr_saved,
                                &[("name", name.as_str()), ("info", canonical.as_str())],
                            ),
                        );
                    }
                    Err(err) => {
                        self.toast_err(err.clone());
                        self.log_err(CAT_UPDATES, err);
                    }
                }
                // Con la dirección nueva, la comprobación se lanza enseguida.
                let mut auto_check = None;
                self.reload_library();
                if let Some(entry) = self.library.iter().find(|e| e.path == path) {
                    if let Some(info) = entry.update_info.clone() {
                        let compare = entry
                            .original_name
                            .clone()
                            .unwrap_or_else(|| file_name_string(&entry.path));
                        auto_check = Some((info, compare, entry.version.clone()));
                    }
                }
                if let Some((info, compare, version)) = auto_check {
                    self.check_updates_for(path, info, compare, version, false);
                }
            }
            Err(err) => {
                let msg = utils::tf(t.upd_addr_invalid, &[("err", err.as_str())]);
                self.toast_err(msg.clone());
                self.log_warn(CAT_UPDATES, msg);
            }
        }
    }

    /// Quita la dirección manual de una AppImage: vuelve a usarse la
    /// información embebida en el archivo, si la tiene.
    pub fn remove_update_address(&mut self, path: &Path) {
        if self
            .upd_overrides
            .remove(&path.display().to_string())
            .is_none()
        {
            return;
        }
        let t = self.t();
        let name = discovery::display_name(path);
        match updstore::save(&self.overrides_path, &self.upd_overrides) {
            Ok(()) => {
                self.toast_ok(utils::tf(t.upd_addr_removed, &[("name", name.as_str())]));
                self.log_info(
                    CAT_UPDATES,
                    utils::tf(t.upd_addr_removed, &[("name", name.as_str())]),
                );
            }
            Err(err) => {
                self.toast_err(err.clone());
                self.log_err(CAT_UPDATES, err);
            }
        }
        self.reload_library();
    }

    /// Tras una actualización que cambió el nombre del archivo, la dirección
    /// manual debe seguir a la AppImage nueva.
    fn migrate_update_override(&mut self, old: &Path, new: &Path) {
        let old_key = old.display().to_string();
        if let Some(raw) = self.upd_overrides.remove(&old_key) {
            self.upd_overrides.insert(new.display().to_string(), raw);
            let _ = updstore::save(&self.overrides_path, &self.upd_overrides);
        }
    }

    pub fn remove_from_menu(&mut self, entry: &AppImageEntry) {
        let Some(desktop) = entry.desktop_path.clone() else {
            return;
        };
        let t = self.t();
        match integration::remove_desktop_integration(
            &desktop,
            &self.dirs.icons_dir,
            &self.dirs.hicolor_dir,
        ) {
            Ok(()) => {
                self.toast_ok(t.toast_menu_removed);
                self.log_ok(
                    CAT_MENU,
                    utils::tf(
                        t.log_menu_removed,
                        &[("path", desktop.display().to_string().as_str())],
                    ),
                );
            }
            Err(err) => {
                self.toast_err(err.clone());
                self.log_err(
                    CAT_MENU,
                    utils::tf(
                        t.log_menu_remove_err,
                        &[
                            ("path", desktop.display().to_string().as_str()),
                            ("err", err.as_str()),
                        ],
                    ),
                );
            }
        }
        self.reload_library();
    }

    pub fn delete_confirmed(&mut self, path: PathBuf) {
        let (map, _) = discovery::scan_our_desktop_entries(&self.dirs.desktops_dir);
        if let Some(desktop) = map.get(&path).cloned() {
            let _ = integration::remove_desktop_integration(
                &desktop,
                &self.dirs.icons_dir,
                &self.dirs.hicolor_dir,
            );
        }
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let trash_root = trash::default_trash_root(&home);
        let name = file_name_string(&path);
        // La dirección manual de esa AppImage ya no tiene dueño: se limpia.
        if self
            .upd_overrides
            .remove(&path.display().to_string())
            .is_some()
        {
            let _ = updstore::save(&self.overrides_path, &self.upd_overrides);
        }
        if self
            .upd_edit
            .as_ref()
            .map(|(edit_path, _)| edit_path == &path)
            .unwrap_or(false)
        {
            self.upd_edit = None;
        }
        let t = self.t();
        match trash::trash_file(&path, &trash_root) {
            Ok(_) => {
                self.toast_ok(utils::tf(t.toast_deleted, &[("name", name.as_str())]));
                self.log_ok(
                    CAT_TRASH,
                    utils::tf(
                        t.log_trashed,
                        &[("path", path.display().to_string().as_str())],
                    ),
                );
            }
            Err(err) => {
                self.toast_err(err.clone());
                self.log_err(
                    CAT_TRASH,
                    utils::tf(
                        t.log_trash_err,
                        &[
                            ("path", path.display().to_string().as_str()),
                            ("err", err.as_str()),
                        ],
                    ),
                );
            }
        }
        self.reload_library();
    }

    // ---------- ciclo de vida ----------

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        let mut active = false;
        let mut finished: Vec<(PathBuf, JobKind, JobStatus)> = Vec::new();
        let mut available_silent: Vec<(PathBuf, String)> = Vec::new();
        let t = self.t();
        for (path, job) in self.jobs.iter_mut() {
            let status = job.status();
            let now = Instant::now();
            if status != job.last_seen {
                job.last_seen = status.clone();
                job.last_change = now;
            }
            // VIGILANTE: un trabajo que lleva demasiado tiempo se aborta con
            // un error claro. Antes, comprobar actualizaciones SIN conexión
            // dejaba la ficha bloqueada eternamente en «Comprobando...»
            // (la resolución DNS puede colgarse mucho más que el timeout
            // de conexión). Aquí el estado siempre se resuelve.
            let stall = now.duration_since(job.last_change);
            let total = now.duration_since(job.started);
            if let Some(timeout_err) = job_timeout_error(job.kind, &status, total, stall) {
                let msg = match job.kind {
                    JobKind::Integrate => utils::tf(
                        t.log_integrate_err,
                        &[
                            ("path", path.display().to_string().as_str()),
                            ("err", timeout_err.as_str()),
                        ],
                    ),
                    JobKind::Analyze => utils::tf(
                        t.log_analyze_err,
                        &[
                            ("path", path.display().to_string().as_str()),
                            ("err", timeout_err.as_str()),
                        ],
                    ),
                    JobKind::Update => utils::tf(
                        t.log_check_err,
                        &[
                            ("path", path.display().to_string().as_str()),
                            ("err", timeout_err.as_str()),
                        ],
                    ),
                };
                self.activity.log(
                    Level::Error,
                    match job.kind {
                        JobKind::Integrate => CAT_INTEGRATE,
                        _ => CAT_UPDATES,
                    },
                    msg,
                );
                let failed = JobStatus::Failed(timeout_err);
                let mut guard = job
                    .status
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *guard = failed.clone();
                drop(guard);
                job.last_seen = failed.clone();
                job.last_change = now;
                if !job.toasted_done {
                    job.toasted_done = true;
                    finished.push((path.clone(), job.kind, failed));
                }
                continue;
            }
            match &status {
                JobStatus::Working(_) | JobStatus::Checking | JobStatus::Progress { .. } => {
                    active = true;
                }
                JobStatus::Available { asset, .. } => {
                    if job.silent && !job.toasted_available {
                        job.toasted_available = true;
                        available_silent.push((path.clone(), asset.clone()));
                    }
                }
                JobStatus::Done(_) | JobStatus::Failed(_) if !job.toasted_done => {
                    job.toasted_done = true;
                    finished.push((path.clone(), job.kind, status));
                }
                _ => {}
            }
        }
        for (path, asset) in available_silent {
            let name = discovery::display_name(&path);
            self.toast_info(utils::tf(
                t.upd_toast_available,
                &[("name", name.as_str()), ("asset", asset.as_str())],
            ));
        }
        let mut must_reload = false;
        for (path, kind, status) in finished {
            match kind {
                JobKind::Integrate => match &status {
                    JobStatus::Done(name) => {
                        self.toast_ok(utils::tf(t.toast_integrated, &[("name", name.as_str())]));
                        must_reload = true;
                    }
                    JobStatus::Failed(err) => {
                        // El error ya queda en el registro con todos los detalles.
                        self.toast_err(err.clone());
                        must_reload = true;
                    }
                    _ => {}
                },
                JobKind::Analyze => match &status {
                    JobStatus::Done(_) => {
                        self.toast_info(t.toast_analyzed);
                        must_reload = true;
                        // Encadenar: si tras analizar la AppImage ya declara
                        // información de actualización (.upd_info del ELF o el
                        // .desktop interno), se comprueba automáticamente.
                        let mut auto_check = None;
                        self.reload_library();
                        if let Some(entry) = self.library.iter().find(|e| e.path == path) {
                            if let Some(info) = entry.update_info.clone() {
                                let compare = entry
                                    .original_name
                                    .clone()
                                    .unwrap_or_else(|| file_name_string(&entry.path));
                                auto_check = Some((info, compare, entry.version.clone()));
                            }
                        }
                        if let Some((info, compare, version)) = auto_check {
                            self.check_updates_for(path.clone(), info, compare, version, false);
                        }
                    }
                    JobStatus::Failed(err) => self.toast_err(err.clone()),
                    _ => {}
                },
                JobKind::Update => match &status {
                    JobStatus::Done(done) => {
                        let name = discovery::display_name(Path::new(done));
                        self.toast_ok(utils::tf(t.toast_updated, &[("name", name.as_str())]));
                        // La dirección manual sigue a la AppImage actualizada.
                        self.migrate_update_override(&path, Path::new(done));
                        must_reload = true;
                    }
                    JobStatus::Failed(err) => self.toast_err(err.clone()),
                    _ => {}
                },
            }
            self.jobs.remove(&path);
        }
        if must_reload {
            self.reload_library();
        }
        if active {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let (dropped, ignored) = ctx.input(|i| {
            let mut ok = Vec::new();
            let mut bad = Vec::new();
            for file in &i.raw.dropped_files {
                if let Some(path) = file.path.clone() {
                    if discovery::is_appimage_name(&path.to_string_lossy()) {
                        ok.push(path);
                    } else {
                        bad.push(path);
                    }
                }
            }
            (ok, bad)
        });
        // Los archivos soltados se entregan una sola vez por fotograma
        // (RawInput::take los consume), así que no hace falta ningún filtro extra.
        if !ignored.is_empty() {
            let t = self.t();
            for path in &ignored {
                self.log_warn(
                    CAT_DROP,
                    utils::tf(
                        t.log_drop_ignored,
                        &[("path", path.display().to_string().as_str())],
                    ),
                );
            }
        }
        if dropped.is_empty() {
            return;
        }
        let list = dropped
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        self.log_info(
            CAT_DROP,
            utils::tf(
                self.t().log_drops,
                &[
                    ("n", dropped.len().to_string().as_str()),
                    ("list", list.as_str()),
                ],
            ),
        );
        ctx.request_repaint();
        for path in dropped {
            self.integrate_dedup(path);
        }
    }

    /// Integra una AppImage evitando duplicados: si ya está en la biblioteca
    /// con menú, avisa; si está sin menú, la integra donde está.
    pub fn integrate_dedup(&mut self, path: PathBuf) {
        if !self.jobs.contains_key(&path) && !self.library.iter().any(|e| e.path == path) {
            self.integrate_path(path);
        } else if !self.jobs.contains_key(&path) {
            // Ya está en la biblioteca: se integra donde está (por si no tiene menú).
            let has_menu = self
                .library
                .iter()
                .find(|e| e.path == path)
                .map(|e| e.desktop_path.is_some())
                .unwrap_or(false);
            if !has_menu {
                self.integrate_path(path);
            } else {
                // Feedback claro: ya estaba integrada, no se duplica.
                let name = discovery::display_name(&path);
                self.toast_info(utils::tf(self.t().int_drop_dup, &[("name", name.as_str())]));
            }
        }
    }

    /// Integra AppImages escritas a mano o pegadas (Ctrl+V) en la pestaña
    /// Integrar. Acepta rutas normales, rutas con «~» y URI file://
    /// (una por línea, como las que copia un explorador de archivos).
    pub fn integrate_from_text(&mut self, raw: &str) {
        let mut accepted: Vec<PathBuf> = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let path = if let Some(rest) = line.strip_prefix("file://") {
                let rest = rest
                    .strip_prefix("localhost")
                    .map(str::to_string)
                    .unwrap_or_else(|| rest.to_string());
                PathBuf::from(utils::percent_decode(&rest))
            } else {
                utils::expand_tilde(line)
            };
            if discovery::is_appimage_name(&path.to_string_lossy()) {
                accepted.push(path);
            } else {
                self.log_warn(
                    CAT_DROP,
                    utils::tf(self.t().log_drop_ignored, &[("path", line)]),
                );
            }
        }
        if accepted.is_empty() {
            self.toast_err(self.t().int_paste_invalid);
            self.log_warn(CAT_INTEGRATE, self.t().log_paste_invalid);
            return;
        }
        let list = accepted
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        self.log_info(
            CAT_INTEGRATE,
            utils::tf(
                self.t().log_paste,
                &[
                    ("n", accepted.len().to_string().as_str()),
                    ("list", list.as_str()),
                ],
            ),
        );
        for path in accepted {
            self.integrate_dedup(path);
        }
    }
}

fn file_name_string(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

impl eframe::App for NaranjaHubApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_jobs(ctx);
        self.handle_drops(ctx);
        crate::ui_common::show_nav(self, ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(crate::theme::BG)
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| match self.tab {
                Tab::Library => self.show_library(ui),
                Tab::Integrate => self.show_integrate(ui, ctx),
                Tab::Updates => self.show_updates(ui),
                Tab::Activity => self.show_activity(ui),
                Tab::Settings => self.show_settings(ui),
                Tab::About => self.show_about(ui),
            });
        crate::ui_common::show_status_bar(self, ctx);
        crate::ui_common::show_toasts(self, ctx);
        crate::ui_common::show_confirm_dialog(self, ctx);
        crate::ui_activity::show_export_dialog(self, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vigilante_comprobaciones_sin_conexion() {
        // Comprobación dentro de plazo → sigue.
        assert!(job_timeout_error(
            JobKind::Update,
            &JobStatus::Checking,
            Duration::from_secs(5),
            Duration::from_secs(5)
        )
        .is_none());
        // Comprobación bloqueada más de CHECK_TIMEOUT → abortar con un
        // mensaje que menciona la conexión (el caso «sin internet»).
        let err = job_timeout_error(
            JobKind::Update,
            &JobStatus::Checking,
            CHECK_TIMEOUT + Duration::from_secs(1),
            CHECK_TIMEOUT + Duration::from_secs(1),
        )
        .expect("debe caducar");
        assert!(err.contains("conexión"), "{err}");
    }

    #[test]
    fn vigilante_descargas_y_copias_por_estancamiento() {
        let prog = JobStatus::Progress {
            label: String::new(),
            got: 500,
            total: Some(1000),
        };
        // Descarga de 10 minutos con progreso ACTIVO → sigue (2 s sin cambios).
        assert!(job_timeout_error(
            JobKind::Update,
            &prog,
            Duration::from_secs(600),
            Duration::from_secs(2)
        )
        .is_none());
        // Descarga estancada más de STALL_TIMEOUT → abortar.
        assert!(job_timeout_error(
            JobKind::Update,
            &prog,
            Duration::from_secs(600),
            STALL_TIMEOUT + Duration::from_secs(1)
        )
        .is_some());
        // Copia grande con progreso vivo → sigue.
        let copia = JobStatus::Progress {
            label: String::new(),
            got: 1,
            total: None,
        };
        assert!(job_timeout_error(
            JobKind::Integrate,
            &copia,
            Duration::from_secs(300),
            Duration::from_secs(1)
        )
        .is_none());
        // Análisis (sin progreso) bloqueado → aborta por estancamiento.
        assert!(job_timeout_error(
            JobKind::Analyze,
            &JobStatus::Working("x".to_string()),
            STALL_TIMEOUT + Duration::from_secs(1),
            STALL_TIMEOUT + Duration::from_secs(1)
        )
        .is_some());
    }

    #[test]
    fn vigilante_estados_terminados_nunca_caducan() {
        // Un trabajo ya terminado o con versión disponible no se toca aunque
        // la ficha siga en pantalla mucho tiempo.
        assert!(job_timeout_error(
            JobKind::Update,
            &JobStatus::UpToDate,
            Duration::from_secs(9999),
            Duration::from_secs(9999)
        )
        .is_none());
        let avail = JobStatus::Available {
            asset: "a".to_string(),
            url: "u".to_string(),
            size: 1,
        };
        assert!(job_timeout_error(
            JobKind::Update,
            &avail,
            Duration::from_secs(9999),
            Duration::from_secs(9999)
        )
        .is_none());
        assert!(job_timeout_error(
            JobKind::Update,
            &JobStatus::Failed("x".to_string()),
            Duration::from_secs(9999),
            Duration::from_secs(9999)
        )
        .is_none());
        // El límite de estancamiento no afecta a una comprobación reciente.
        assert!(job_timeout_error(
            JobKind::Update,
            &JobStatus::Checking,
            Duration::from_secs(2),
            Duration::from_secs(2)
        )
        .is_none());
    }
}
