// NaranjaHub — pestaña Acerca de: autoría y licencia GNU GPL v3.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{self, Align, RichText, Rounding, ScrollArea, Vec2};

use crate::app::NaranjaHubApp;
use crate::theme;
use crate::ui_common::badge;

impl NaranjaHubApp {
    pub fn show_about(&mut self, ui: &mut egui::Ui) {
        let t = self.t();
        theme::heading(ui, t.ab_title);
        ui.add_space(8.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    if let Some(logo) = self.logo.clone() {
                        ui.add(
                            egui::Image::new(egui::load::SizedTexture::new(
                                logo.id(),
                                Vec2::splat(132.0),
                            ))
                            .rounding(Rounding::same(28.0)),
                        );
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(t.app_title)
                            .size(30.0)
                            .strong()
                            .color(theme::ORANGE),
                    );
                    ui.label(RichText::new(t.tagline).color(theme::MUTED));
                    ui.add_space(6.0);
                    badge(
                        ui,
                        &format!("v{}", env!("CARGO_PKG_VERSION")),
                        theme::ORANGE,
                    );
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(t.ab_created_by)
                            .strong()
                            .size(17.0)
                            .color(theme::TEXT),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new(t.ab_copyright).small().color(theme::MUTED));
                    ui.add_space(14.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(560.0, 0.0),
                        egui::Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.label(RichText::new(t.ab_desc).color(theme::MUTED));
                        },
                    );
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(t.ab_license_heading)
                            .strong()
                            .color(theme::ORANGE),
                    );
                    ui.add_space(4.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(560.0, 0.0),
                        egui::Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.label(RichText::new(t.ab_license_text).color(theme::MUTED).small());
                        },
                    );
                    ui.add_space(8.0);
                    ui.hyperlink_to(
                        t.ab_license_link,
                        "https://www.gnu.org/licenses/gpl-3.0.html",
                    );
                    ui.label(RichText::new(t.ab_license_note).small().color(theme::MUTED));
                    ui.add_space(14.0);
                    ui.label(RichText::new(t.ab_tech).small().color(theme::MUTED));
                    ui.add_space(16.0);
                });
            });
    }
}
