// NaranjaHub — Administrador de AppImages para Linux.
// Copyright (C) 2026 Leyter Vázquez. V
// SPDX-License-Identifier: GPL-3.0-only
//
// Este programa es software libre: puede redistribuirlo y/o modificarlo
// bajo los términos de la GNU General Public License versión 3.
// Vea el archivo LICENSE para el texto completo.

mod app;
mod browser;
mod config;
mod core;
mod i18n;
mod theme;
mod ui_about;
mod ui_activity;
mod ui_common;
mod ui_integrate;
mod ui_library;
mod ui_settings;
mod ui_updates;

use std::path::PathBuf;

fn print_help() {
    println!(
        "NaranjaHub v{} — Administrador de AppImages",
        env!("CARGO_PKG_VERSION")
    );
    println!("Creado por Leyter Vázquez. V — Licencia GNU GPL v3");
    println!();
    println!("Uso:");
    println!("  naranjahub                  Inicia la interfaz gráfica");
    println!(
        "  naranjahub --selftest       Ejecuta simulaciones internas (sin pantalla ni Internet)"
    );
    println!("  naranjahub --inspect RUTA   Muestra los metadatos detectados de una AppImage");
    println!("  naranjahub --version        Muestra la versión");
    println!("  naranjahub --help           Muestra esta ayuda");
    println!();
    println!("Variables de entorno:");
    println!("  NARANJAHUB_BACKEND=x11|wayland|auto   Fuerza el motor gráfico (por defecto: auto)");
}

fn inspect_appimage(path: &std::path::Path) -> i32 {
    use crate::core::integration;
    use crate::core::updater;
    if !path.is_file() {
        eprintln!("No existe el archivo: {}", path.display());
        return 2;
    }
    println!("Archivo: {}", path.display());
    let meta = integration::extract_metadata(path);
    match meta.name {
        Some(name) => println!("  Nombre: {name}"),
        None => println!("  Nombre: (no encontrado en el .desktop interno)"),
    }
    if let Some(version) = &meta.version {
        println!("  Versión: {version}");
    }
    if let Some(categories) = &meta.categories {
        println!("  Categorías: {categories}");
    }
    match &meta.icon_data {
        Some(data) => println!(
            "  Icono: {} bytes ({})",
            data.len(),
            meta.icon_ext.as_deref().unwrap_or("?")
        ),
        None => println!("  Icono: (no encontrado)"),
    }
    match &meta.update_information {
        Some(info) => {
            println!("  Información de actualización: {info}");
            match updater::parse_update_information(info) {
                Some(parsed) => println!(
                    "  → GitHub: {}/{} (patrón «{}»)",
                    parsed.owner, parsed.repo, parsed.pattern
                ),
                None => println!("  → formato no reconocido para comprobación automática"),
            }
        }
        None => println!("  Información de actualización: (no encontrada)"),
    }
    if meta.update_information.is_some() {
        0
    } else {
        1
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        None => run_gui(),
        Some("--version") | Some("-V") => {
            println!(
                "NaranjaHub v{} — Creado por Leyter Vázquez. V — GNU GPL v3",
                env!("CARGO_PKG_VERSION")
            );
        }
        Some("--help") | Some("-h") => print_help(),
        Some("--inspect") => match args.get(1) {
            Some(path) => {
                let code = inspect_appimage(std::path::Path::new(path));
                std::process::exit(code);
            }
            None => {
                eprintln!("Falta la ruta: naranjahub --inspect RUTA.AppImage");
                std::process::exit(2);
            }
        },
        Some("--selftest") => {
            let code = core::selftest::run();
            std::process::exit(code);
        }
        Some(other) => {
            eprintln!("Argumento no reconocido: {other}");
            println!();
            print_help();
            std::process::exit(2);
        }
    }
}

fn run_gui() {
    // Preferencia de backend: variable de entorno > configuración > auto.
    // «auto» intenta X11 primero (XWayland incluido) porque es el único
    // backend con arrastrar y soltar funcional, y si no hay X11 disponible
    // arranca en Wayland nativo para que la app funcione igualmente.
    let config_path = dirs::config_dir()
        .map(|d| d.join("NaranjaHub/config.json"))
        .unwrap_or_else(|| PathBuf::from(".naranjahub/config.json"));
    let pref = match std::env::var("NARANJAHUB_BACKEND") {
        Ok(value) => config::normalize_backend(&value),
        Err(_) => config::load(&config_path).backend_pref(),
    };
    let attempts: &[&str] = match pref.as_str() {
        "x11" => &["x11"],
        "wayland" => &["wayland"],
        _ => &["x11", "wayland"],
    };
    let mut last_error = None;
    for backend in attempts {
        match run_on_backend(backend) {
            Ok(()) => return,
            Err(err) => {
                eprintln!("NaranjaHub: no se pudo arrancar con {backend}: {err}");
                last_error = Some(err);
            }
        }
    }
    eprintln!(
        "NaranjaHub no pudo iniciarse: {}",
        last_error.unwrap_or_default()
    );
    eprintln!("Comprueba que hay un entorno gráfico activo (X11 o Wayland).");
    std::process::exit(1);
}

/// Arranca la interfaz forzando un backend gráfico concreto.
fn run_on_backend(backend: &str) -> Result<(), String> {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("NaranjaHub")
        .with_inner_size([1120.0, 700.0])
        .with_min_inner_size([940.0, 600.0]);
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let backend_name = backend.to_string();
    let backend_for_builder = backend_name.clone();
    let options = eframe::NativeOptions {
        viewport,
        // Se fuerza el backend ANTES de crear el bucle de eventos.
        event_loop_builder: Some(Box::new(move |builder| {
            if backend_for_builder == "x11" {
                winit::platform::x11::EventLoopBuilderExtX11::with_x11(builder);
            } else {
                winit::platform::wayland::EventLoopBuilderExtWayland::with_wayland(builder);
            }
        })),
        ..Default::default()
    };
    let backend_for_app = backend_name.clone();
    let result = eframe::run_native(
        "NaranjaHub",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(app::NaranjaHubApp::new(cc, &backend_for_app)) as Box<dyn eframe::App>)
        }),
    );
    result.map_err(|err| err.to_string())
}

fn load_window_icon() -> Option<eframe::egui::IconData> {
    let bytes = include_bytes!("../assets/icon-256.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(eframe::egui::IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}
