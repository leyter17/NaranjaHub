// NaranjaHub — pestaña Biblioteca.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{self, Align, Margin, RichText, ScrollArea, Stroke, Vec2};

use crate::app::{NaranjaHubApp, Sort, Tab};
use crate::core::discovery::AppImageEntry;
use crate::core::utils;
use crate::i18n::Strings;
use crate::theme;
use crate::ui_common::badge;

impl NaranjaHubApp {
    pub fn show_library(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        ui.horizontal(|ui| {
            theme::heading(ui, t.lib_title);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.add(theme::ghost_button(t.lib_btn_open_folder)).clicked() {
                    let dir = self.dirs.apps_dir.clone();
                    self.open_in_file_manager(&dir);
                }
                if ui.add(theme::ghost_button(t.lib_btn_refresh)).clicked() {
                    self.reload_library();
                }
                egui::ComboBox::from_id_salt("naranjahub_sort")
                    .selected_text(format!("{} {}", t.lib_sort_label, sort_label(self.sort, t)))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort, Sort::Name, t.lib_sort_name);
                        ui.selectable_value(&mut self.sort, Sort::Date, t.lib_sort_date);
                        ui.selectable_value(&mut self.sort, Sort::Size, t.lib_sort_size);
                    });
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(t.lib_search)
                        .desired_width(200.0),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(8.0);

        if self.library.is_empty() {
            ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                ui.add_space(ui.available_height() * 0.22);
                ui.label(
                    RichText::new(t.lib_empty_title)
                        .heading()
                        .color(theme::TEXT),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(t.lib_empty_msg).color(theme::MUTED));
                ui.add_space(14.0);
                if ui
                    .add(theme::primary_button(t.lib_btn_go_integrate))
                    .clicked()
                {
                    self.tab = Tab::Integrate;
                }
            });
            return;
        }

        let query = self.search.to_lowercase();
        let visible: Vec<AppImageEntry> = self
            .library
            .iter()
            .filter(|entry| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        let empty_results = visible.is_empty();
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if empty_results {
                    ui.label(
                        RichText::new(utils::tf(t.lib_no_results, &[("q", query.as_str())]))
                            .color(theme::MUTED),
                    );
                }
                for entry in visible {
                    self.library_card(ui, entry);
                    ui.add_space(6.0);
                }
            });
    }

    fn library_card(&mut self, ui: &mut egui::Ui, entry: AppImageEntry) {
        let t = self.t();
        egui::Frame::none()
            .fill(theme::CARD)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    // Icono real extraído de la AppImage; si aún no hay, el genérico.
                    let icon = self
                        .icon_for(ui.ctx(), &entry.path)
                        .or_else(|| self.generic_icon.clone());
                    if let Some(icon) = icon {
                        ui.add(
                            egui::Image::new(egui::load::SizedTexture::new(
                                icon.id(),
                                Vec2::splat(42.0),
                            ))
                            .rounding(egui::Rounding::same(9.0)),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&entry.name)
                                    .strong()
                                    .size(16.0)
                                    .color(theme::TEXT),
                            );
                            if entry.desktop_path.is_some() {
                                badge(ui, t.lib_badge_integrated, theme::OK);
                            } else {
                                badge(ui, t.lib_badge_not, theme::MUTED);
                            }
                            if !entry.executable {
                                badge(ui, t.lib_badge_noexec, theme::WARN);
                            }
                        });
                        let size = utils::human_size(entry.size);
                        let date = utils::fmt_date(entry.mtime, self.config.lang);
                        let mut subtitle = format!("{size} · {date}");
                        if let Some(version) = &entry.version {
                            subtitle = format!("{subtitle} · v{version}");
                        }
                        ui.label(RichText::new(subtitle).small().color(theme::MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.menu_button(t.lib_btn_more, |ui| {
                            if entry.desktop_path.is_none() {
                                if ui.button(t.lib_opt_integrate).clicked() {
                                    ui.close_menu();
                                    self.integrate_path(entry.path.clone());
                                }
                            } else if ui.button(t.lib_opt_unintegrate).clicked() {
                                ui.close_menu();
                                self.remove_from_menu(&entry);
                            }
                            if ui.button(t.lib_opt_reveal).clicked() {
                                ui.close_menu();
                                let path = entry.path.clone();
                                self.open_in_file_manager(&path);
                            }
                            if ui.button(t.lib_opt_delete).clicked() {
                                ui.close_menu();
                                if self.config.confirm_deletes {
                                    self.pending_delete = Some(entry.path.clone());
                                } else {
                                    self.delete_confirmed(entry.path.clone());
                                }
                            }
                        });
                        if ui.add(theme::primary_button(t.lib_btn_run)).clicked() {
                            let path = entry.path.clone();
                            self.launch(&path);
                        }
                    });
                });
            });
    }
}

fn sort_label(sort: Sort, t: &Strings) -> &'static str {
    match sort {
        Sort::Name => t.lib_sort_name,
        Sort::Date => t.lib_sort_date,
        Sort::Size => t.lib_sort_size,
    }
}
