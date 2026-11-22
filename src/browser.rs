// NaranjaHub — navegador de archivos integrado.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only
//
// Navegador propio para no depender de diálogos nativos: funciona igual en
// GNOME, KDE Plasma, COSMIC y XFCE, y soporta carpetas o archivos .AppImage.

use std::path::{Component, PathBuf};

use eframe::egui::{self, RichText, ScrollArea, Vec2};

use crate::core::discovery;
use crate::core::utils;
use crate::i18n::Strings;
use crate::theme;

#[derive(Clone, Debug)]
pub struct BrowserEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct Browser {
    pub current: PathBuf,
    pub entries: Vec<BrowserEntry>,
    pub selected: Option<PathBuf>,
    pub error: Option<String>,
}

impl Browser {
    pub fn at_home() -> Self {
        let current = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut browser = Self {
            current,
            entries: Vec::new(),
            selected: None,
            error: None,
        };
        browser.refresh();
        browser
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;
        let Ok(read) = std::fs::read_dir(&self.current) else {
            self.error = Some(format!("no se puede leer {}", self.current.display()));
            return;
        };
        for entry in read.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
                continue;
            };
            if path.is_dir() {
                self.entries.push(BrowserEntry {
                    name,
                    is_dir: true,
                    size: 0,
                });
            } else if discovery::is_appimage_name(&name) {
                self.entries.push(BrowserEntry {
                    name,
                    is_dir: false,
                    size: meta.len(),
                });
            }
        }
        self.entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current.parent().map(|p| p.to_path_buf()) {
            self.current = parent;
            self.selected = None;
            self.refresh();
        }
    }

    pub fn go_home(&mut self) {
        if let Some(home) = dirs::home_dir() {
            self.current = home;
            self.selected = None;
            self.refresh();
        }
    }

    pub fn open(&mut self, name: &str) {
        let next = self.current.join(name);
        if next.is_dir() {
            self.current = next;
            self.selected = None;
            self.refresh();
        } else if next.is_file() {
            // Clic sobre el archivo ya marcado → lo desmarca (y otro clic
            // lo vuelve a marcar).
            if self.selected.as_ref() == Some(&next) {
                self.selected = None;
            } else {
                self.selected = Some(next);
            }
        }
    }

    /// Dibuja el navegador. `dirs_only` muestra solo carpetas (elegir carpeta).
    pub fn render(&mut self, ui: &mut egui::Ui, t: &Strings, dirs_only: bool) {
        // Migas de pan (con rutas propias para poder reasignar current)
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let mut chain = PathBuf::new();
            let crumbs: Vec<(String, PathBuf)> = self
                .current
                .components()
                .map(|comp| {
                    chain.push(comp);
                    let label = match &comp {
                        Component::RootDir => "/".to_string(),
                        _ => comp.as_os_str().to_string_lossy().to_string(),
                    };
                    (label, chain.clone())
                })
                .collect();
            let total = crumbs.len();
            for (index, (label, path)) in crumbs.into_iter().enumerate() {
                let label = if index == total - 1 {
                    format!("» {label}")
                } else {
                    label
                };
                if ui.add(theme::ghost_button(&label)).clicked() {
                    self.current = path;
                    self.selected = None;
                    self.refresh();
                }
            }
        });

        ui.horizontal(|ui| {
            if ui.add(theme::ghost_button(t.br_up)).clicked() {
                self.go_up();
            }
            if ui.add(theme::ghost_button(t.br_home)).clicked() {
                self.go_home();
            }
            if ui.add(theme::ghost_button(t.br_refresh)).clicked() {
                self.selected = None;
                self.refresh();
            }
        });

        ScrollArea::vertical()
            .id_salt("naranjahub_browser")
            .max_height(240.0)
            .show(ui, |ui| {
                if let Some(err) = &self.error {
                    ui.label(RichText::new(err).color(theme::DANGER).small());
                    return;
                }
                if self.entries.is_empty() {
                    ui.label(RichText::new(t.br_empty).color(theme::MUTED).small());
                    return;
                }
                for entry in self.entries.clone() {
                    if dirs_only && !entry.is_dir {
                        continue;
                    }
                    let path = self.current.join(&entry.name);
                    let is_selected = self.selected.as_ref() == Some(&path);
                    let label = if entry.is_dir {
                        format!("{}/", entry.name)
                    } else {
                        entry.name.clone()
                    };
                    ui.horizontal(|ui| {
                        let (dot, _) =
                            ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            dot,
                            3.0,
                            if entry.is_dir {
                                theme::MUTED
                            } else {
                                theme::ORANGE
                            },
                        );
                        let response = if entry.is_dir {
                            ui.add(theme::ghost_button(&label))
                        } else if is_selected {
                            ui.add(theme::secondary_button(&label))
                        } else {
                            ui.add(theme::ghost_button(&label))
                        };
                        if response.clicked() {
                            self.open(&entry.name);
                        }
                        if !entry.is_dir {
                            ui.label(
                                RichText::new(utils::human_size(entry.size))
                                    .small()
                                    .color(theme::MUTED),
                            );
                        }
                    });
                }
            });

        if dirs_only {
            ui.label(RichText::new(t.br_dirs_hint).small().color(theme::MUTED));
        }
    }
}
