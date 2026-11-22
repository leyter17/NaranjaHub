// NaranjaHub — simulaciones internas (naranjahub --selftest).
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// Comprueba la lógica principal de la aplicación sin interfaz gráfica y
// sin conexión a Internet: utilidades, i18n, configuración, descubrimiento,
// integración, accesos directos, papelera y actualizaciones.

use std::path::{Path, PathBuf};

use crate::config;
use crate::core::activity::{ActivityLog, Level, ReportContext, CAT_INTEGRATE, CAT_LAUNCH};
use crate::core::discovery;
use crate::core::integration::{self, DirPaths, IntegrateOptions};
use crate::core::trash;
use crate::core::updater;
use crate::core::updstore;
use crate::core::utils;
use crate::i18n::{Lang, EN, ES};

/// PNG real de 1×1 para las AppImage simuladas.
const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/// Crea una "AppImage" simulada: un script con permisos de ejecución que
/// responde a --appimage-extract igual que una AppImage tipo 2 real
/// (extrae .DirIcon como enlace al icono y un .desktop interno con nombre).
fn make_fake_appimage(dir: &Path, file_name: &str) -> PathBuf {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(file_name);
    let script = format!(
        "#!/bin/sh\n\
        if [ \"$1\" = \"--appimage-extract\" ]; then\n\
            mkdir -p squashfs-root/usr/share/applications \
              squashfs-root/usr/share/icons/hicolor/256x256/apps\n\
            printf '%s' \"{png_b64}\" | base64 -d > \\
              squashfs-root/usr/share/icons/hicolor/256x256/apps/demo-sim.png\n\
            ln -sf usr/share/icons/hicolor/256x256/apps/demo-sim.png \\
              squashfs-root/.DirIcon\n\
            cat > squashfs-root/demo-sim.desktop <<'EOF'\n\
[Desktop Entry]\n\
Type=Application\n\
Name=Demo Simulada\n\
Exec=demo-sim\n\
Icon=demo-sim\n\
X-AppImage-Version=3.1\n\
X-AppImage-UpdateInformation=gh-releases-zsync|demo|sim|latest|*.AppImage\n\
EOF\n\
            cp squashfs-root/demo-sim.desktop \\
              squashfs-root/usr/share/applications/demo-sim.desktop\n\
            exit 0\n\
        fi\n\
        echo \"ejecutando demo-sim\"\n\
        exit 0\n",
        png_b64 = PNG_1X1,
    );
    std::fs::write(&path, script).expect("escribir AppImage simulada");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

struct Checker {
    passed: u32,
    failed: u32,
}

impl Checker {
    fn check(&mut self, name: &str, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.passed += 1;
                println!("  [OK]    {name}");
            }
            Err(err) => {
                self.failed += 1;
                println!("  [FALLO] {name}: {err}");
            }
        }
    }
}

fn expect_eq(actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("se obtuvo «{actual}», se esperaba «{expected}»"))
    }
}

fn temp_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("naranjahub-selftest-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn run() -> i32 {
    println!(
        "NaranjaHub v{} — simulaciones internas",
        env!("CARGO_PKG_VERSION")
    );
    println!("(no se requiere pantalla ni conexión a Internet)");
    println!();
    let mut c = Checker {
        passed: 0,
        failed: 0,
    };
    let root = temp_root();

    c.check(
        "tamaños legibles",
        || -> Result<(), String> {
            expect_eq(utils::human_size(0).as_str(), "0 B")?;
            expect_eq(utils::human_size(999).as_str(), "999 B")?;
            expect_eq(utils::human_size(1000).as_str(), "1.0 KB")?;
            expect_eq(utils::human_size(1536).as_str(), "1.5 KB")?;
            expect_eq(utils::human_size(1_048_576).as_str(), "1.0 MB")?;
            Ok(())
        }(),
    );

    c.check(
        "fechas en español e inglés",
        || -> Result<(), String> {
            expect_eq(utils::fmt_date(0, Lang::Es).as_str(), "1 ene 1970")?;
            expect_eq(utils::fmt_date(0, Lang::En).as_str(), "Jan 1, 1970")?;
            expect_eq(
                utils::fmt_date(951_782_400, Lang::Es).as_str(),
                "29 feb 2000",
            )?;
            Ok(())
        }(),
    );

    c.check(
        "limpieza de nombres",
        || -> Result<(), String> {
            expect_eq(
                utils::clean_display_name("MiApp-1.2-x86_64").as_str(),
                "MiApp-1.2",
            )?;
            expect_eq(
                utils::sanitize_filename("Mi App (2026)!").as_str(),
                "Mi_App_2026",
            )?;
            Ok(())
        }(),
    );

    c.check(
        "textos en español e inglés completos",
        || -> Result<(), String> {
            for (es, en) in [
                (ES.app_title, EN.app_title),
                (ES.tagline, EN.tagline),
                (ES.nav_library, EN.nav_library),
                (ES.ab_created_by, EN.ab_created_by),
                (ES.ab_license_text, EN.ab_license_text),
                (ES.cfm_delete_body, EN.cfm_delete_body),
            ] {
                if es.trim().is_empty() || en.trim().is_empty() {
                    return Err("hay textos vacíos".to_string());
                }
            }
            if ES.nav_library == EN.nav_library {
                return Err("los dos idiomas no deben tener el mismo texto".to_string());
            }
            for lang in Lang::all() {
                if !lang.strings().ab_created_by.contains("Leyter Vázquez. V") {
                    return Err("el texto de autor no menciona a Leyter Vázquez. V".to_string());
                }
            }
            Ok(())
        }(),
    );

    c.check(
        "guardado y carga de configuración",
        || -> Result<(), String> {
            let path = root.join("config.json");
            let cfg = config::Config {
                lang: Lang::En,
                apps_dir: "/tmp/apps".to_string(),
                check_updates_on_start: true,
                ..Default::default()
            };
            config::save(&path, &cfg)?;
            if config::load(&path) != cfg {
                return Err("la configuración leída no coincide".to_string());
            }
            if config::load(&root.join("inexistente.json")).lang != Lang::Es {
                return Err("el idioma por defecto debe ser español".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "parseo de información de actualización (gh-releases-zsync)",
        || -> Result<(), String> {
            let info = updater::parse_update_information(
                "gh-releases-zsync|proyecto|repo|latest|*.AppImage",
            )
            .ok_or("no se pudo parsear")?;
            expect_eq(info.owner.as_str(), "proyecto")?;
            expect_eq(info.repo.as_str(), "repo")?;
            expect_eq(info.pattern.as_str(), "*.AppImage")?;
            if updater::parse_update_information("otro-formato|x|y|z").is_some() {
                return Err("aceptó una cadena inválida".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "comodines de nombres de archivos",
        || -> Result<(), String> {
            for (name, pattern, expected) in [
                ("Foo-1.2.AppImage", "*.AppImage", true),
                ("Foo-1.2.AppImage", "Foo-?.AppImage", false),
                ("foo.tar.gz", "FOO*.GZ", true),
                ("appimage", "a*image", true),
            ] {
                if updater::match_wildcard(name, pattern) != expected {
                    return Err(format!("{name} vs {pattern}"));
                }
            }
            Ok(())
        }(),
    );

    c.check(
        "selección de assets en GitHub Releases",
        || -> Result<(), String> {
            let assets = vec![
                updater::ReleaseAsset {
                    name: "Demo-1.2.0.AppImage".to_string(),
                    url: "u1".to_string(),
                    size: 1,
                },
                updater::ReleaseAsset {
                    name: "Demo-1.2.0.AppImage.blockmap".to_string(),
                    url: "u2".to_string(),
                    size: 1,
                },
                updater::ReleaseAsset {
                    name: "fuentes.tar.gz".to_string(),
                    url: "u3".to_string(),
                    size: 2,
                },
            ];
            let found = updater::find_matching_asset(&assets, "Demo-*.AppImage")
                .ok_or("no encontró ningún asset")?;
            expect_eq(found.name.as_str(), "Demo-1.2.0.AppImage")?;
            if updater::find_matching_asset(&assets, "NoExiste-*.AppImage").is_some() {
                return Err("encontró un asset que no coincide".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "descubrimiento de AppImages",
        || -> Result<(), String> {
            let dir = root.join("apps");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            std::fs::write(dir.join("Uno.AppImage"), "x").map_err(|e| e.to_string())?;
            std::fs::write(dir.join("dos.appimage"), "x").map_err(|e| e.to_string())?;
            std::fs::write(dir.join("leeme.txt"), "x").map_err(|e| e.to_string())?;
            let found = discovery::scan_apps_dir(&dir);
            if found.len() != 2 {
                return Err(format!("encontró {} de 2", found.len()));
            }
            Ok(())
        }(),
    );

    c.check(
        "integración de una AppImage de prueba",
        || -> Result<(), String> {
            let src_dir = root.join("origen");
            let dirs = DirPaths {
                apps_dir: root.join("gestion"),
                icons_dir: root.join("iconos"),
                hicolor_dir: root.join("hicolor"),
                desktops_dir: root.join("accesos"),
            };
            std::fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;
            let src = src_dir.join("Demo.AppImage");
            std::fs::write(&src, "contenido-de-prueba".repeat(100)).map_err(|e| e.to_string())?;
            let outcome =
                integration::integrate(&src, &IntegrateOptions::default(), &dirs, &|_, _| {})?;
            if !outcome.new_path.exists() {
                return Err("no se copió a la carpeta gestionada".to_string());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&outcome.new_path)
                    .map_err(|e| e.to_string())?
                    .permissions()
                    .mode();
                if mode & 0o111 == 0 {
                    return Err("sin permiso de ejecución".to_string());
                }
            }
            let desktop = outcome.desktop_path.ok_or("no se creó el acceso directo")?;
            if !desktop.exists() {
                return Err("el archivo .desktop no existe".to_string());
            }
            let (map, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
            if !map.contains_key(&outcome.new_path) {
                return Err("el acceso directo no apunta a la AppImage".to_string());
            }
            // Integrar de nuevo debe usar un nombre único, sin sobrescribir.
            let outcome2 =
                integration::integrate(&src, &IntegrateOptions::default(), &dirs, &|_, _| {})?;
            if outcome2.new_path == outcome.new_path {
                return Err("debería usar un nombre único".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "escritura y lectura de accesos directos .desktop",
        || -> Result<(), String> {
            let dir = root.join("accesos2");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let meta = integration::ExtractedMeta {
                name: Some("DemoApp".to_string()),
                ..Default::default()
            };
            let dest = integration::write_desktop_entry(
                &dir,
                "demoapp",
                "DemoApp",
                Path::new("/opt/Demo.AppImage"),
                "demoapp",
                &meta,
            )?;
            let keys = integration::parse_desktop_file(
                &std::fs::read_to_string(&dest).map_err(|e| e.to_string())?,
            );
            expect_eq(
                keys.get("Type").map(String::as_str).unwrap_or(""),
                "Application",
            )?;
            expect_eq(
                keys.get("X-NaranjaHub-Source")
                    .map(String::as_str)
                    .unwrap_or(""),
                "/opt/Demo.AppImage",
            )?;
            expect_eq(
                keys.get("Icon").map(String::as_str).unwrap_or(""),
                "demoapp",
            )?;
            Ok(())
        }(),
    );

    c.check(
        "AppImage simulada: icono y nombre correctos en el acceso directo",
        || -> Result<(), String> {
            let dirs = DirPaths {
                apps_dir: root.join("sim-gestion"),
                icons_dir: root.join("sim-iconos"),
                hicolor_dir: root.join("sim-hicolor"),
                desktops_dir: root.join("sim-accesos"),
            };
            let src = make_fake_appimage(&root.join("sim-origen"), "DemoSim-3.1-x86_64.AppImage");
            let outcome =
                integration::integrate(&src, &IntegrateOptions::default(), &dirs, &|_, _| {})?;
            // El nombre sale del .desktop interno de la AppImage
            expect_eq(outcome.app_name.as_str(), "Demo Simulada")?;
            let desktop = outcome.desktop_path.ok_or("no se creó el acceso")?;
            let text = std::fs::read_to_string(&desktop).map_err(|e| e.to_string())?;
            let keys = integration::parse_desktop_file(&text);
            expect_eq(
                keys.get("Name").map(String::as_str).unwrap_or(""),
                "Demo Simulada",
            )?;
            // Icon= por NOMBRE instalado en el tema hicolor (no una ruta rota)
            let icon_ref = keys.get("Icon").map(String::as_str).unwrap_or("");
            if icon_ref.is_empty() || icon_ref.contains('/') || icon_ref == "naranjahub" {
                return Err(format!("Icon= incorrecto: {icon_ref}"));
            }
            expect_eq(icon_ref, integration::slug_for(&outcome.new_path).as_str())?;
            let slug = icon_ref;
            let mut found_icon = false;
            for size in [
                "16x16",
                "22x22",
                "24x24",
                "32x32",
                "36x36",
                "48x48",
                "64x64",
                "72x72",
                "96x96",
                "128x128",
                "192x192",
                "256x256",
                "512x512",
                "1024x1024",
            ] {
                if dirs
                    .hicolor_dir
                    .join(format!("{size}/apps/{slug}.png"))
                    .is_file()
                {
                    found_icon = true;
                }
            }
            if !found_icon {
                return Err("el icono no quedó instalado en el tema hicolor".to_string());
            }
            if !dirs.icons_dir.join(format!("{slug}.png")).is_file() {
                return Err("falta la copia interna del icono".to_string());
            }
            // La biblioteca detecta la integración por el .desktop
            let (map, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
            if !map.contains_key(&outcome.new_path) {
                return Err("la biblioteca no ve el acceso directo".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "quitar del menú borra el acceso directo y todos sus iconos",
        || -> Result<(), String> {
            let dirs = DirPaths {
                apps_dir: root.join("rm-gestion"),
                icons_dir: root.join("rm-iconos"),
                hicolor_dir: root.join("rm-hicolor"),
                desktops_dir: root.join("rm-accesos"),
            };
            let src = make_fake_appimage(&root.join("rm-origen"), "Quitada.AppImage");
            let outcome =
                integration::integrate(&src, &IntegrateOptions::default(), &dirs, &|_, _| {})?;
            let desktop = outcome.desktop_path.ok_or("no se creó el acceso")?;
            let slug = integration::slug_for(&outcome.new_path);
            integration::remove_desktop_integration(&desktop, &dirs.icons_dir, &dirs.hicolor_dir)?;
            if desktop.exists() {
                return Err("el .desktop sigue existiendo".to_string());
            }
            if dirs.icons_dir.join(format!("{slug}.png")).exists() {
                return Err("la copia interna del icono no se borró".to_string());
            }
            if dirs
                .hicolor_dir
                .join(format!("16x16/apps/{slug}.png"))
                .exists()
            {
                return Err("el icono del tema hicolor no se borró".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "papelera freedesktop",
        || -> Result<(), String> {
            let trash_root = root.join("Trash");
            let src = root.join("archivo-a-borrar.AppImage");
            std::fs::write(&src, "datos").map_err(|e| e.to_string())?;
            let trashed = trash::trash_file(&src, &trash_root)?;
            if !trashed.file_path.exists() {
                return Err("el archivo no está en la papelera".to_string());
            }
            if src.exists() {
                return Err("el original sigue existiendo".to_string());
            }
            let info = std::fs::read_to_string(&trashed.info_path).map_err(|e| e.to_string())?;
            if !info.starts_with("[Trash Info]") {
                return Err("información de papelera inválida".to_string());
            }
            if !info.contains("Path=") || !info.contains("DeletionDate=") {
                return Err("faltan campos Path o DeletionDate".to_string());
            }
            if info.contains('Z') {
                return Err("la fecha de borrado debe ser hora local, sin Z".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "papelera del sistema respeta $XDG_DATA_HOME",
        || -> Result<(), String> {
            let data_home = root.join("datos-xdg");
            let home = PathBuf::from("/home/prueba");
            // Con la variable de entorno, la papelera es $XDG_DATA_HOME/Trash
            std::env::set_var("XDG_DATA_HOME", &data_home);
            if trash::default_trash_root(&home) != data_home.join("Trash") {
                std::env::remove_var("XDG_DATA_HOME");
                return Err("no se usó XDG_DATA_HOME".to_string());
            }
            let trash_root = trash::default_trash_root(&home);
            let src = root.join("con-xdg.AppImage");
            std::fs::write(&src, "contenido").map_err(|e| e.to_string())?;
            let trashed = trash::trash_file(&src, &trash_root)?;
            std::env::remove_var("XDG_DATA_HOME");
            // El archivo aparece donde lo buscan GNOME, KDE, COSMIC y XFCE
            if !trashed.file_path.starts_with(data_home.join("Trash/files")) {
                return Err("no fue a $XDG_DATA_HOME/Trash/files".to_string());
            }
            let info = std::fs::read_to_string(&trashed.info_path).map_err(|e| e.to_string())?;
            let expected = format!("Path={}", src.display());
            if !info.contains(&expected) {
                return Err("el .trashinfo no guarda la ruta original".to_string());
            }
            // Sin la variable, la papelera clásica
            if trash::default_trash_root(&home) != Path::new("/home/prueba/.local/share/Trash") {
                return Err("sin XDG_DATA_HOME debe usarse ~/.local/share/Trash".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "registro de actividad: escritura, recarga y limpieza",
        || -> Result<(), String> {
            let path = root.join("logs").join("actividad.log");
            let log = ActivityLog::new(path.clone());
            log.log(Level::Info, "app", "evento uno");
            log.log(Level::Error, CAT_INTEGRATE, "evento dos con error");
            let snap = log.snapshot();
            if snap.len() != 2 {
                return Err(format!("se esperaban 2 eventos, hay {}", snap.len()));
            }
            if snap[1].level != Level::Error {
                return Err("el segundo evento no es de error".to_string());
            }
            // Un registro nuevo debe cargar el historial del archivo.
            let log2 = ActivityLog::new(path.clone());
            if log2.snapshot().len() != 2 {
                return Err("el historial no se recargó del archivo".to_string());
            }
            log2.clear();
            if !ActivityLog::new(path).snapshot().is_empty() {
                return Err("tras limpiar el archivo debería quedar vacío".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "exportación del informe de fallos y errores",
        || -> Result<(), String> {
            let log = ActivityLog::new(root.join("logs2").join("actividad.log"));
            log.log(Level::Ok, CAT_INTEGRATE, "integración correcta");
            log.log(Level::Error, CAT_LAUNCH, "no se pudo lanzar la app");
            let context = ReportContext {
                config_path: "/tmp/config.json".to_string(),
                apps_dir: "/tmp/apps".to_string(),
            };
            let report = log.export_report(&root.join("informes"), &ES, &context)?;
            if !report.exists() {
                return Err("no se creó el archivo del informe".to_string());
            }
            let text = std::fs::read_to_string(&report).map_err(|e| e.to_string())?;
            if !text.contains("[ERROR]") || !text.contains("no se pudo lanzar la app") {
                return Err("el informe no incluye los errores registrados".to_string());
            }
            if !text.contains("Leyter Vázquez. V") || !text.contains("GNU GPL v3") {
                return Err("el informe no incluye autoría y licencia".to_string());
            }
            // El mismo informe en inglés debe usar los textos en inglés.
            let report_en = log.export_report(&root.join("informes"), &EN, &context)?;
            let text_en = std::fs::read_to_string(&report_en).map_err(|e| e.to_string())?;
            if !text_en.contains("FULL ACTIVITY LOG") {
                return Err("el informe en inglés no usa sus textos".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "acceso directo duplicado evitado al reintegrar",
        || -> Result<(), String> {
            let dir = root.join("accesos3");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let meta = integration::ExtractedMeta::default();
            let src = Path::new("/opt/Repetida.AppImage");
            let first = integration::write_desktop_entry(
                &dir, "repetida", "Repetida", src, "repetida", &meta,
            )?;
            let second = integration::write_desktop_entry(
                &dir, "repetida", "Repetida", src, "repetida", &meta,
            )?;
            if first != second {
                return Err("se creó un segundo acceso para la misma AppImage".to_string());
            }
            let count = std::fs::read_dir(&dir)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok())
                .count();
            if count != 1 {
                return Err(format!("hay {count} accesos, debería haber 1"));
            }
            Ok(())
        }(),
    );

    c.check(
        "lectura de .upd_info en ELF y comparación de versiones",
        || -> Result<(), String> {
            // Un ELF con la sección .upd_info (como los que crea appimagetool).
            let elf = updater::build_test_elf(
                ".upd_info",
                b"gh-releases-zsync|autor|repositorio|latest|MiApp-*.AppImage\0",
            );
            let path = root.join("con-updinfo.AppImage");
            std::fs::write(&path, &elf).map_err(|e| e.to_string())?;
            let info = updater::read_upd_info(&path).ok_or("no se leyó la sección .upd_info")?;
            let parsed = updater::parse_update_information(&info)
                .ok_or("la información leída no se pudo parsear")?;
            expect_eq(parsed.owner.as_str(), "autor")?;
            expect_eq(parsed.repo.as_str(), "repositorio")?;
            expect_eq(parsed.pattern.as_str(), "MiApp-*.AppImage")?;
            // Un ELF sin sección → None (no debe romperse).
            let vacio = root.join("sin-updinfo.AppImage");
            std::fs::write(&vacio, updater::build_test_elf(".otra", b"x"))
                .map_err(|e| e.to_string())?;
            if updater::read_upd_info(&vacio).is_some() {
                return Err("devolvió información que no existe".to_string());
            }
            // Comparación de versiones: no ofrecer «nueva versión» si es la misma.
            if updater::is_newer_asset(
                "MiApp-1.2.0-x86_64.AppImage",
                Some("1.2.0"),
                "Mi (2).AppImage",
            ) {
                return Err("marcó como nueva la misma versión".to_string());
            }
            if !updater::is_newer_asset(
                "MiApp-1.3.0-x86_64.AppImage",
                Some("1.2.0"),
                "MiApp-1.2.0-x86_64.AppImage",
            ) {
                return Err("no detectó la versión 1.3.0 como nueva".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "dirección de actualización manual: URL, usuario/repo y gh-releases-zsync",
        || -> Result<(), String> {
            // URL completa del repositorio: el patrón se deduce del nombre.
            let info = updater::parse_manual_address(
                "https://github.com/usuario/repositorio",
                "DemoApp-1.2.0-x86_64.AppImage",
            )?;
            expect_eq(info.owner.as_str(), "usuario")?;
            expect_eq(info.repo.as_str(), "repositorio")?;
            expect_eq(info.pattern.as_str(), "DemoApp-*-x86_64.AppImage")?;
            // usuario/repo a secas.
            let info2 = updater::parse_manual_address("usuario/otro-repo", "App.AppImage")?;
            expect_eq(info2.owner.as_str(), "usuario")?;
            expect_eq(info2.repo.as_str(), "otro-repo")?;
            expect_eq(info2.pattern.as_str(), "App.AppImage")?;
            // Cadena completa gh-releases-zsync: conserva su propio patrón.
            let info3 = updater::parse_manual_address(
                "gh-releases-zsync|u|r|latest|Mi-*.AppImage",
                "Demo.AppImage",
            )?;
            expect_eq(info3.pattern.as_str(), "Mi-*.AppImage")?;
            // La cadena canónica se puede volver a parsear.
            let canon = updater::canonical_update_information(&info);
            expect_eq(
                canon.as_str(),
                "gh-releases-zsync|usuario|repositorio|latest|DemoApp-*-x86_64.AppImage",
            )?;
            if updater::parse_update_information(&canon).is_none() {
                return Err("la cadena canónica no se puede re-parsear".to_string());
            }
            // Direcciones inválidas → error claro, no pánico.
            if updater::parse_manual_address("", "x.AppImage").is_ok() {
                return Err("aceptó una dirección vacía".to_string());
            }
            if updater::parse_manual_address("https://github.com/solo-owner", "x.AppImage").is_ok()
            {
                return Err("aceptó una dirección sin repositorio".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "almacén persistente de direcciones manuales (updates.json)",
        || -> Result<(), String> {
            let path = root.join("updates.json");
            let mut map = updstore::UpdateOverrides::new();
            map.insert(
                "/a/Demo.AppImage".to_string(),
                "gh-releases-zsync|u|r|latest|Demo-*.AppImage".to_string(),
            );
            updstore::save(&path, &map)?;
            let loaded = updstore::load(&path);
            if loaded.get("/a/Demo.AppImage").map(String::as_str)
                != Some("gh-releases-zsync|u|r|latest|Demo-*.AppImage")
            {
                return Err("la dirección guardada no se recuperó".to_string());
            }
            // Un archivo corrupto no debe romper el arranque.
            std::fs::write(&path, "{no json").map_err(|e| e.to_string())?;
            if !updstore::load(&path).is_empty() {
                return Err("un archivo corrupto debería dar un almacén vacío".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "AppImage con .upd_info en ELF: la información llega al acceso directo",
        || -> Result<(), String> {
            let dirs = DirPaths {
                apps_dir: root.join("elf-gestion"),
                icons_dir: root.join("elf-iconos"),
                hicolor_dir: root.join("elf-hicolor"),
                desktops_dir: root.join("elf-accesos"),
            };
            // AppImage "realista" en lo que importa: ELF tipo 2 con .upd_info
            // pero sin soporte --appimage-extract; la información debe llegar
            // igualmente al acceso directo para poder comprobar updates.
            let src = root.join("elf-origen");
            std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
            let path = src.join("Realista-1.0.0-x86_64.AppImage");
            std::fs::write(
                &path,
                updater::build_test_elf(
                    ".upd_info",
                    b"gh-releases-zsync|fabri|realista|latest|Realista-*.AppImage\0",
                ),
            )
            .map_err(|e| e.to_string())?;
            integration::set_executable(&path)?;
            let outcome =
                integration::integrate(&path, &IntegrateOptions::default(), &dirs, &|_, _| {})?;
            let desktop = outcome.desktop_path.ok_or("no se creó el acceso")?;
            let text = std::fs::read_to_string(&desktop).map_err(|e| e.to_string())?;
            let keys = integration::parse_desktop_file(&text);
            let stored = keys
                .get("X-NaranjaHub-UpdateInformation")
                .ok_or("el acceso directo no guardó la información de actualización")?;
            if updater::parse_update_information(stored).is_none() {
                return Err("la información guardada no se puede parsear".to_string());
            }
            // La biblioteca la ve y queda lista para comprobar en GitHub.
            let (_, infos) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
            let info = infos
                .get(&outcome.new_path)
                .ok_or("la biblioteca no encontró la información de actualización")?;
            expect_eq(info.owner.as_str(), "fabri")?;
            expect_eq(info.repo.as_str(), "realista")?;
            Ok(())
        }(),
    );

    c.check(
        "integración por ruta pegada y URI file://",
        || -> Result<(), String> {
            // La decodificación de URI file:// (espacios y acentos codificados).
            expect_eq(
                utils::percent_decode("/a b/Mi%20App.AppImage").as_str(),
                "/a b/Mi App.AppImage",
            )?;
            expect_eq(
                utils::percent_decode("file:///home/usuario/Jos%C3%A9.AppImage")
                    .trim_start_matches("file://"),
                "/home/usuario/José.AppImage",
            )?;
            Ok(())
        }(),
    );

    c.check(
        "instalación de actualización con respaldo y restauración",
        || -> Result<(), String> {
            let dirs = DirPaths {
                apps_dir: root.join("upd-gestion"),
                icons_dir: root.join("upd-iconos"),
                hicolor_dir: root.join("upd-hicolor"),
                desktops_dir: root.join("upd-accesos"),
            };
            // Versión antigua integrada.
            let src = root.join("upd-origen");
            std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
            let old_src = src.join("DemoApp-1.0.0-x86_64.AppImage");
            std::fs::write(&old_src, "contenido-v1").map_err(|e| e.to_string())?;
            let outcome =
                integration::integrate(&old_src, &IntegrateOptions::default(), &dirs, &|_, _| {})?;
            let old = outcome.new_path;
            // Actualización con el MISMO nombre: sustitución en sitio.
            let new_file = root.join("descarga-misma.part");
            std::fs::write(&new_file, "contenido-v2").map_err(|e| e.to_string())?;
            let dest =
                integration::apply_update(&old, &new_file, "DemoApp-1.0.0-x86_64.AppImage", &dirs)?;
            expect_eq(
                std::fs::read_to_string(&dest)
                    .map_err(|e| e.to_string())?
                    .as_str(),
                "contenido-v2",
            )?;
            if dest != old {
                return Err("con el mismo nombre debía sustituirse en sitio".to_string());
            }
            let (map, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
            if !map.contains_key(&dest) {
                return Err("el acceso directo dejó de apuntar al archivo".to_string());
            }
            // Fallo simulado: el archivo nuevo desaparece → error y sin cambios.
            let missing = root.join("no-existe.part");
            let err =
                integration::apply_update(&old, &missing, "DemoApp-1.0.0-x86_64.AppImage", &dirs);
            if err.is_ok() {
                return Err("aceptó una descarga inexistente".to_string());
            }
            if std::fs::read_to_string(&old).map_err(|e| e.to_string())? != "contenido-v2" {
                return Err("el archivo cambió tras un fallo".to_string());
            }
            // Actualización con nombre nuevo: entra el archivo nuevo y sale el viejo.
            let new_file2 = root.join("descarga-nueva.part");
            std::fs::write(&new_file2, "contenido-v3").map_err(|e| e.to_string())?;
            let dest2 = integration::apply_update(
                &old,
                &new_file2,
                "DemoApp-1.1.0-x86_64.AppImage",
                &dirs,
            )?;
            expect_eq(
                std::fs::read_to_string(&dest2)
                    .map_err(|e| e.to_string())?
                    .as_str(),
                "contenido-v3",
            )?;
            if old.exists() {
                return Err("la versión vieja no se eliminó".to_string());
            }
            let (map2, _) = discovery::scan_our_desktop_entries(&dirs.desktops_dir);
            if !map2.contains_key(&dest2) {
                return Err("el archivo nuevo no quedó integrado".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "vigilante: comprobar sin conexión no se queda bloqueado",
        || -> Result<(), String> {
            use crate::app::{job_timeout_error, JobKind, JobStatus, CHECK_TIMEOUT, STALL_TIMEOUT};
            use std::time::Duration;
            // Comprobación recién lanzada → sigue (no hay error aún).
            if job_timeout_error(
                JobKind::Update,
                &JobStatus::Checking,
                Duration::from_secs(5),
                Duration::from_secs(5),
            )
            .is_some()
            {
                return Err("abortó una comprobación recién lanzada".to_string());
            }
            // Comprobación bloqueada más del límite total → abortar con
            // mensaje claro (el caso «sin conexión a internet»).
            let err = job_timeout_error(
                JobKind::Update,
                &JobStatus::Checking,
                CHECK_TIMEOUT + Duration::from_secs(1),
                CHECK_TIMEOUT + Duration::from_secs(1),
            )
            .ok_or("no abortó la comprobación que superó el límite")?;
            if !err.contains("conexión") {
                return Err(format!("mensaje poco claro: {err}"));
            }
            // Descarga con progreso ACTIVO durante 10 minutos → sigue.
            let prog = JobStatus::Progress {
                label: String::new(),
                got: 500,
                total: Some(1000),
            };
            if job_timeout_error(
                JobKind::Update,
                &prog,
                Duration::from_secs(600),
                Duration::from_secs(2),
            )
            .is_some()
            {
                return Err("abortó una descarga con progreso activo".to_string());
            }
            // Descarga ESTANCADA más del límite → abortar.
            if job_timeout_error(
                JobKind::Update,
                &prog,
                Duration::from_secs(600),
                STALL_TIMEOUT + Duration::from_secs(1),
            )
            .is_none()
            {
                return Err("no abortó una descarga estancada".to_string());
            }
            // Un estado terminado (Al día, Nueva versión, Error) nunca caduca.
            if job_timeout_error(
                JobKind::Update,
                &JobStatus::UpToDate,
                Duration::from_secs(9999),
                Duration::from_secs(9999),
            )
            .is_some()
            {
                return Err("caducó un trabajo ya terminado".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "errores de red sin conexión se explican con claridad",
        || -> Result<(), String> {
            if !updater::is_offline_kind(ureq::ErrorKind::Dns) {
                return Err("DNS no se marca como fallo de conexión".to_string());
            }
            if !updater::is_offline_kind(ureq::ErrorKind::ConnectionFailed) {
                return Err("conexión rechazada no se marca como fallo de conexión".to_string());
            }
            if !updater::is_offline_kind(ureq::ErrorKind::Io) {
                return Err("error de E/S de red no se marca como fallo de conexión".to_string());
            }
            if updater::is_offline_kind(ureq::ErrorKind::BadStatus) {
                return Err("un estado HTTP raro no es «sin conexión»".to_string());
            }
            Ok(())
        }(),
    );

    c.check(
        "analizar una AppImage borrada falla sin crear accesos huecos",
        || -> Result<(), String> {
            let dirs = DirPaths {
                apps_dir: root.join("fantasma-gestion"),
                icons_dir: root.join("fantasma-iconos"),
                hicolor_dir: root.join("fantasma-hicolor"),
                desktops_dir: root.join("fantasma-accesos"),
            };
            let fantasma = root.join("fantasma-gestion/NoExiste-1.0-x86_64.AppImage");
            // El archivo NO se crea en disco (simula que lo borraron fuera de la app).
            let result = integration::refresh_integration(&fantasma, &dirs);
            if result.is_ok() {
                return Err("analizar un archivo inexistente debería fallar".to_string());
            }
            // No debe quedar ningún acceso directo creado a partir del fantasma.
            let restos = std::fs::read_dir(&dirs.desktops_dir)
                .map(|d| d.count())
                .unwrap_or(0);
            if restos > 0 {
                return Err(format!("quedaron {restos} accesos huecos"));
            }
            Ok(())
        }(),
    );

    c.check(
        "limpieza de archivos temporales de simulación",
        || -> Result<(), String> {
            if root.exists() {
                std::fs::remove_dir_all(&root).map_err(|e| format!("no se pudo limpiar: {e}"))?;
            }
            Ok(())
        }(),
    );

    println!();
    println!("Resultado: {} correctas, {} fallidas", c.passed, c.failed);
    if c.failed == 0 {
        println!("TODAS LAS SIMULACIONES PASARON");
        0
    } else {
        println!("HAY SIMULACIONES FALLIDAS");
        1
    }
}
