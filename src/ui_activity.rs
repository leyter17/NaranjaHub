// NaranjaHub — pestaña Registro: actividad detallada e informe de fallos.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{
    self, Align, Color32, Margin, Pos2, RichText, Rounding, ScrollArea, Stroke, Vec2,
};

use crate::app::{ActivityFilter, NaranjaHubApp};
use crate::browser::Browser;
use crate::core::activity::{
    ActivityEntry, Level, ReportContext, CAT_APP, CAT_DROP, CAT_INTEGRATE, CAT_LAUNCH, CAT_LIBRARY,
    CAT_MENU, CAT_REPORT, CAT_SETTINGS, CAT_TRASH, CAT_UPDATES,
};
use crate::core::utils;
use crate::theme;
use crate::ui_common::badge;

impl NaranjaHubApp {
    pub fn show_activity(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        theme::heading(ui, t.act_title);
        ui.label(RichText::new(t.act_desc).color(theme::MUTED).small());
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let entries = self.activity.snapshot();
        let warnings = entries.iter().filter(|e| e.level == Level::Warn).count();
        let errors = entries.iter().filter(|e| e.level == Level::Error).count();

        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.activity_filter,
                ActivityFilter::All,
                t.act_filter_all,
            );
            ui.radio_value(
                &mut self.activity_filter,
                ActivityFilter::Problems,
                t.act_filter_problems,
            );
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.add(theme::primary_button(t.act_btn_export)).clicked() {
                    self.report_browser = Some(Browser::at_home());
                }
                if ui.add(theme::ghost_button(t.act_btn_clear)).clicked() {
                    self.activity.clear();
                    self.log_info(CAT_APP, t.log_cleared);
                    self.toast_ok(t.act_cleared);
                }
            });
        });
        ui.label(
            RichText::new(utils::tf(
                t.act_summary,
                &[
                    ("n", entries.len().to_string().as_str()),
                    ("w", warnings.to_string().as_str()),
                    ("e", errors.to_string().as_str()),
                ],
            ))
            .small()
            .color(theme::MUTED),
        );
        ui.add_space(4.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let filter = self.activity_filter;
                let rows: Vec<ActivityEntry> = entries
                    .iter()
                    .rev()
                    .filter(|e| match filter {
                        ActivityFilter::All => true,
                        ActivityFilter::Problems => {
                            e.level == Level::Warn || e.level == Level::Error
                        }
                    })
                    .cloned()
                    .collect();
                if rows.is_empty() {
                    ui.label(RichText::new(t.act_empty).color(theme::MUTED));
                    return;
                }
                for entry in rows {
                    self.activity_row(ui, entry);
                    ui.add_space(2.0);
                }
            });
    }

    fn activity_row(&mut self, ui: &mut egui::Ui, entry: ActivityEntry) {
        let t = self.t();
        let color = match entry.level {
            Level::Info => theme::MUTED,
            Level::Ok => theme::OK,
            Level::Warn => theme::WARN,
            Level::Error => theme::DANGER,
        };
        let level_word = match entry.level {
            Level::Info => t.act_level_info,
            Level::Ok => t.act_level_ok,
            Level::Warn => t.act_level_warn,
            Level::Error => t.act_level_error,
        };
        let category_word = match entry.category {
            CAT_APP => t.cat_app,
            CAT_LIBRARY => t.cat_library,
            CAT_INTEGRATE => t.cat_integrate,
            CAT_MENU => t.cat_menu,
            CAT_TRASH => t.cat_trash,
            CAT_SETTINGS => t.cat_settings,
            CAT_UPDATES => t.cat_updates,
            CAT_DROP => t.cat_drop,
            CAT_REPORT => t.cat_report,
            CAT_LAUNCH => t.cat_launch,
            _ => entry.category,
        };
        egui::Frame::none()
            .fill(if entry.level == Level::Error {
                color.gamma_multiply(0.10)
            } else {
                Color32::TRANSPARENT
            })
            .stroke(if entry.level == Level::Error {
                Stroke::new(1.0_f32, color.gamma_multiply(0.5))
            } else {
                Stroke::NONE
            })
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, color);
                    ui.label(
                        RichText::new(utils::fmt_datetime_local(entry.time))
                            .small()
                            .color(theme::MUTED),
                    );
                    badge(ui, level_word, color);
                    ui.label(RichText::new(category_word).small().color(theme::ORANGE));
                    ui.label(RichText::new(&entry.message).color(theme::TEXT));
                });
            });
    }
}

/// Diálogo para elegir la carpeta donde se exporta el informe de fallos.
pub fn show_export_dialog(app: &mut NaranjaHubApp, ctx: &egui::Context) {
    if app.report_browser.is_none() {
        return;
    }
    let t = app.t();
    egui::Area::new(egui::Id::new("naranjahub_report_dim"))
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(ctx.screen_rect(), 0.0, Color32::from_black_alpha(150));
        });
    egui::Area::new(egui::Id::new("naranjahub_report"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(14.0))
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(560.0);
                    ui.label(
                        RichText::new(t.act_export_title)
                            .heading()
                            .color(theme::TEXT),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new(t.act_export_hint).color(theme::MUTED).small());
                    ui.add_space(10.0);
                    let dir = app
                        .report_browser
                        .as_ref()
                        .map(|b| b.current.clone())
                        .unwrap_or_default();
                    if let Some(browser) = app.report_browser.as_mut() {
                        browser.render(ui, t, true);
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.add(theme::primary_button(t.act_export_btn)).clicked() {
                            let context = ReportContext {
                                config_path: app.config_path.display().to_string(),
                                apps_dir: app.dirs.apps_dir.display().to_string(),
                            };
                            match app.activity.export_report(&dir, app.t(), &context) {
                                Ok(path) => {
                                    app.report_browser = None;
                                    app.toast_ok(utils::tf(
                                        t.act_report_done,
                                        &[("path", path.display().to_string().as_str())],
                                    ));
                                    app.log_ok(
                                        CAT_REPORT,
                                        utils::tf(
                                            t.log_report,
                                            &[("path", path.display().to_string().as_str())],
                                        ),
                                    );
                                }
                                Err(err) => {
                                    app.toast_err(utils::tf(
                                        t.act_report_failed,
                                        &[("err", err.as_str())],
                                    ));
                                    app.log_err(
                                        CAT_REPORT,
                                        utils::tf(t.log_report_err, &[("err", err.as_str())]),
                                    );
                                }
                            }
                        }
                        if ui.add(theme::ghost_button(t.cfm_cancel)).clicked() {
                            app.report_browser = None;
                        }
                    });
                });
        });
}
