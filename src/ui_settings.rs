// NaranjaHub — pestaña Ajustes.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{self, Margin, RichText, ScrollArea, Stroke};

use crate::app::NaranjaHubApp;
use crate::browser::Browser;
use crate::config::Config;
use crate::core::utils;
use crate::i18n::Lang;
use crate::theme;

impl NaranjaHubApp {
    pub fn show_settings(&mut self, ui: &mut egui::Ui) {
        // Con scroll ningún botón queda fuera de la pantalla en ventanas pequeñas.
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.settings_content(ui);
            });
    }

    fn settings_content(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        theme::heading(ui, t.set_title);
        ui.add_space(6.0);

        // Idioma
        card(ui, |ui| {
            ui.label(RichText::new(t.set_lang).strong().color(theme::TEXT));
            let before = self.config.lang;
            ui.horizontal(|ui| {
                for lang in Lang::all() {
                    let label = match lang {
                        Lang::Es => t.set_lang_es,
                        Lang::En => t.set_lang_en,
                    };
                    ui.radio_value(&mut self.config.lang, lang, label);
                }
            });
            if before != self.config.lang {
                let lang_text = match self.config.lang {
                    Lang::Es => t.set_lang_es,
                    Lang::En => t.set_lang_en,
                };
                self.log_info(
                    crate::core::activity::CAT_SETTINGS,
                    utils::tf(t.log_lang, &[("lang", lang_text)]),
                );
                self.finish_settings_change();
            }
        });

        // Carpeta de aplicaciones
        card(ui, |ui| {
            ui.label(RichText::new(t.set_apps_dir).strong().color(theme::TEXT));
            let current = utils::expand_tilde(&self.config.apps_dir);
            let current_text = current.display().to_string();
            ui.label(
                RichText::new(utils::tf(
                    t.set_dir_current,
                    &[("dir", current_text.as_str())],
                ))
                .small()
                .color(theme::MUTED),
            );
            ui.horizontal(|ui| {
                let label = if self.settings_browser.is_some() {
                    t.set_btn_hide
                } else {
                    t.set_btn_change
                };
                if ui.add(theme::ghost_button(label)).clicked() {
                    if self.settings_browser.is_some() {
                        self.settings_browser = None;
                    } else {
                        self.settings_browser = Some(Browser::at_home());
                    }
                }
                if ui.add(theme::ghost_button(t.set_btn_reset_dir)).clicked() {
                    self.config.apps_dir = Config::default().apps_dir;
                    self.apply_apps_dir_change();
                }
                if ui
                    .add(theme::ghost_button(t.set_btn_open_apps_dir))
                    .clicked()
                {
                    let dir = self.dirs.apps_dir.clone();
                    self.open_in_file_manager(&dir);
                }
            });
            if self.settings_browser.is_some() {
                ui.add_space(6.0);
                let dir = self
                    .settings_browser
                    .as_ref()
                    .map(|b| b.current.clone())
                    .unwrap_or_default();
                if let Some(browser) = self.settings_browser.as_mut() {
                    browser.render(ui, t, true);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(theme::primary_button(t.set_btn_use_dir)).clicked() {
                        self.config.apps_dir = dir.display().to_string();
                        self.apply_apps_dir_change();
                        self.settings_browser = None;
                    }
                });
            }
        });

        // Motor gráfico (backend)
        card(ui, |ui| {
            ui.label(RichText::new(t.set_backend).strong().color(theme::TEXT));
            let current = self.config.backend_pref();
            let mut chosen = current.clone();
            ui.horizontal(|ui| {
                ui.radio_value(&mut chosen, "auto".to_string(), t.set_backend_auto);
                ui.radio_value(&mut chosen, "x11".to_string(), t.set_backend_x11);
                ui.radio_value(&mut chosen, "wayland".to_string(), t.set_backend_wayland);
            });
            ui.label(
                RichText::new(t.set_backend_hint)
                    .small()
                    .color(theme::MUTED),
            );
            if chosen != current {
                self.config.backend = chosen.clone();
                self.log_info(
                    crate::core::activity::CAT_SETTINGS,
                    utils::tf(t.log_backend_change, &[("backend", chosen.as_str())]),
                );
                self.finish_settings_change();
            }
        });

        // Comportamiento
        card(ui, |ui| {
            ui.label(RichText::new(t.set_behavior).strong().color(theme::TEXT));
            if ui
                .checkbox(
                    &mut self.config.check_updates_on_start,
                    t.set_opt_check_start,
                )
                .changed()
            {
                self.finish_settings_change();
            }
            if ui
                .checkbox(&mut self.config.confirm_deletes, t.set_opt_confirm_del)
                .changed()
            {
                self.finish_settings_change();
            }
        });

        // Registro de actividad
        card(ui, |ui| {
            ui.label(RichText::new(t.act_title).strong().color(theme::TEXT));
            let logs_path = self.activity.file_path();
            let logs_text = logs_path.display().to_string();
            ui.label(
                RichText::new(utils::tf(t.set_logs_path, &[("path", logs_text.as_str())]))
                    .small()
                    .color(theme::MUTED),
            );
            ui.horizontal(|ui| {
                if ui.add(theme::ghost_button(t.set_btn_open_logs)).clicked() {
                    let dir = logs_path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_default();
                    self.open_in_file_manager(&dir);
                }
            });
        });

        // Datos
        card(ui, |ui| {
            ui.label(RichText::new(t.set_data).strong().color(theme::TEXT));
            let config_text = self.config_path.display().to_string();
            ui.label(
                RichText::new(utils::tf(
                    t.set_config_path,
                    &[("path", config_text.as_str())],
                ))
                .small()
                .color(theme::MUTED),
            );
            ui.horizontal(|ui| {
                if ui.add(theme::ghost_button(t.set_btn_open_config)).clicked() {
                    let dir = self
                        .config_path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_default();
                    self.open_in_file_manager(&dir);
                }
                if ui.add(theme::danger_button(t.set_btn_reset_all)).clicked() {
                    self.config = Config::default();
                    let dir_text = utils::expand_tilde(&self.config.apps_dir)
                        .display()
                        .to_string();
                    self.apply_apps_dir_change();
                    self.log_ok(
                        crate::core::activity::CAT_SETTINGS,
                        utils::tf(t.log_reset, &[("dir", dir_text.as_str())]),
                    );
                    self.toast_ok(t.toast_config_reset);
                }
            });
        });
    }

    fn finish_settings_change(&mut self) {
        if let Err(err) = self.save_config() {
            self.toast_err(err.clone());
            self.log_err(
                crate::core::activity::CAT_SETTINGS,
                utils::tf(self.t().log_settings_err, &[("err", err.as_str())]),
            );
        } else {
            self.toast_ok(self.t().toast_settings_saved);
            self.log_ok(
                crate::core::activity::CAT_SETTINGS,
                self.t().log_settings_saved,
            );
        }
    }

    fn apply_apps_dir_change(&mut self) {
        let new_dir = utils::expand_tilde(&self.config.apps_dir);
        self.dirs.apps_dir = new_dir.clone();
        let _ = std::fs::create_dir_all(&new_dir);
        self.log_info(
            crate::core::activity::CAT_SETTINGS,
            utils::tf(
                self.t().log_apps_dir,
                &[("dir", new_dir.display().to_string().as_str())],
            ),
        );
        self.reload_library();
        self.finish_settings_change();
    }
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(theme::CARD)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
    ui.add_space(8.0);
}
