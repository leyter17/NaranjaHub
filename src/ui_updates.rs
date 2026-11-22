// NaranjaHub — pestaña Actualizaciones.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{self, Align, Margin, RichText, ScrollArea, Stroke};

use crate::app::{JobKind, JobStatus, NaranjaHubApp};
use crate::core::discovery::AppImageEntry;
use crate::core::utils;
use crate::theme;
use crate::ui_common::badge;

impl NaranjaHubApp {
    pub fn show_updates(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        ui.horizontal(|ui| {
            theme::heading(ui, t.upd_title);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.add(theme::primary_button(t.upd_btn_check_all)).clicked() {
                    self.check_all(false);
                }
            });
        });
        ui.label(RichText::new(t.upd_desc).color(theme::MUTED).small());
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);

        if self.library.is_empty() {
            ui.label(RichText::new(t.lib_empty_title).color(theme::MUTED));
            return;
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let entries = self.library.clone();
                for entry in entries {
                    self.updates_row(ui, &entry);
                    ui.add_space(6.0);
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t.upd_no_info_hint)
                        .small()
                        .color(theme::MUTED),
                );
            });
    }

    fn updates_row(&mut self, ui: &mut egui::Ui, entry: &AppImageEntry) {
        let (status, kind) = self
            .jobs
            .get(&entry.path)
            .map(|job| (Some(job.status()), Some(job.kind)))
            .unwrap_or((None, None));
        egui::Frame::none()
            .fill(theme::CARD)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&entry.name).strong().color(theme::TEXT));
                        let size = utils::human_size(entry.size);
                        ui.label(RichText::new(size).small().color(theme::MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        self.updates_status(ui, entry, status, kind);
                    });
                });
                // Dirección de actualización manual (definir, editar o quitar).
                self.updates_address_line(ui, entry);
            });
    }

    /// Línea inferior de cada ficha: editor de la dirección de actualización
    /// manual, la dirección guardada (con botones editar/quitar) o el botón
    /// para definirla cuando aún no existe.
    fn updates_address_line(&mut self, ui: &mut egui::Ui, entry: &AppImageEntry) {
        let t = self.t();
        let editing = self
            .upd_edit
            .as_ref()
            .map(|(path, _)| path == &entry.path)
            .unwrap_or(false);
        if editing {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            ui.label(RichText::new(t.upd_addr_label).small().color(theme::MUTED));
            if let Some((_, text)) = self.upd_edit.as_mut() {
                ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text("https://github.com/usuario/repo")
                        .desired_width(ui.available_width()),
                );
            }
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.add(theme::primary_button(t.upd_addr_save)).clicked() {
                    if let Some((path, text)) = self.upd_edit.take() {
                        self.set_update_address(path, &text);
                    }
                }
                if ui.add(theme::ghost_button(t.upd_addr_cancel)).clicked() {
                    self.upd_edit = None;
                }
            });
            ui.label(RichText::new(t.upd_addr_hint).small().color(theme::MUTED));
            return;
        }
        let override_raw = self
            .upd_overrides
            .get(&entry.path.display().to_string())
            .cloned();
        ui.add_space(4.0);
        // Con salto de línea para que la cadena y los botones no se salgan
        // de la tarjeta en ventanas estrechas.
        ui.horizontal_wrapped(|ui| match override_raw {
            Some(raw) => {
                badge(ui, t.upd_addr_badge, theme::ORANGE);
                ui.label(RichText::new(&raw).small().color(theme::MUTED));
                if ui.add(theme::ghost_button(t.upd_btn_edit_addr)).clicked() {
                    self.upd_edit = Some((entry.path.clone(), raw));
                }
                if ui.add(theme::ghost_button(t.upd_btn_remove_addr)).clicked() {
                    self.remove_update_address(&entry.path);
                }
            }
            None => {
                if ui.add(theme::ghost_button(t.upd_btn_set_addr)).clicked() {
                    self.upd_edit = Some((entry.path.clone(), String::new()));
                }
            }
        });
    }

    fn updates_status(
        &mut self,
        ui: &mut egui::Ui,
        entry: &AppImageEntry,
        status: Option<JobStatus>,
        kind: Option<JobKind>,
    ) {
        let t = self.t();
        match status {
            Some(JobStatus::Working(_)) | Some(JobStatus::Checking) => {
                ui.label(
                    RichText::new(t.upd_status_checking)
                        .small()
                        .color(theme::ORANGE),
                );
            }
            Some(JobStatus::Progress { got, total, .. }) => {
                // Una descarga (JobKind::Update) muestra «Descargando...»;
                // un progreso sin kind (defensivo) muestra «Comprobando...».
                let label = if kind == Some(JobKind::Update) {
                    t.upd_status_downloading
                } else {
                    t.upd_status_checking
                };
                ui.label(RichText::new(label).small().color(theme::ORANGE));
                if let Some(total) = total.filter(|tt| *tt > 0) {
                    ui.add(
                        egui::ProgressBar::new((got as f32 / total as f32).clamp(0.0, 1.0))
                            .desired_height(6.0)
                            .fill(theme::ORANGE_HOVER),
                    );
                }
            }
            Some(JobStatus::UpToDate) => {
                badge(ui, t.upd_status_uptodate, theme::OK);
            }
            Some(JobStatus::Available { asset, url, size }) => {
                badge(
                    ui,
                    utils::tf(t.upd_status_available, &[("asset", asset.as_str())]).as_str(),
                    theme::WARN,
                );
                if ui.add(theme::primary_button(t.upd_btn_update)).clicked() {
                    self.download_update(entry.path.clone(), url, asset, size);
                }
            }
            Some(JobStatus::Done(_)) => {
                badge(ui, t.upd_status_analyzed, theme::OK);
            }
            Some(JobStatus::Failed(err)) => {
                ui.label(
                    RichText::new(utils::tf(t.upd_status_failed, &[("err", err.as_str())]))
                        .small()
                        .color(theme::DANGER),
                );
            }
            None => {
                if entry.update_info.is_none() {
                    ui.label(
                        RichText::new(t.upd_status_no_info)
                            .small()
                            .color(theme::MUTED),
                    );
                    if ui.add(theme::ghost_button(t.upd_btn_analyze)).clicked() {
                        self.analyze_path(entry.path.clone());
                    }
                } else {
                    let compare = entry.original_name.clone().unwrap_or_else(|| {
                        entry
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    });
                    ui.label(RichText::new(t.upd_status_idle).small().color(theme::MUTED));
                    if ui.add(theme::ghost_button(t.upd_btn_check)).clicked() {
                        if let Some(info) = entry.update_info.clone() {
                            self.check_updates_for(
                                entry.path.clone(),
                                info,
                                compare,
                                entry.version.clone(),
                                false,
                            );
                        }
                    }
                }
            }
        }
    }
}
