// NaranjaHub — componentes comunes de la interfaz.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use eframe::egui::{self, Align, Color32, Margin, Pos2, RichText, Rounding, Stroke, Vec2};

use crate::app::{NaranjaHubApp, Tab};
use crate::core::utils;
use crate::theme;

#[derive(Clone, Copy, PartialEq)]
pub enum ToastKind {
    Ok,
    Error,
    Info,
}

#[derive(Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub msg: String,
    pub born: std::time::Instant,
}

/// Etiqueta pequeña con borde de color (Integrada, Al día, errores...).
pub fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::proportional(12.0), color);
    let pad_x = 9.0;
    let pad_y = 4.0;
    let size = Vec2::new(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect(
        rect,
        20.0,
        color.gamma_multiply(0.16),
        Stroke::new(1.0_f32, color),
    );
    ui.painter()
        .galley(rect.min + egui::vec2(pad_x, pad_y), galley, color);
}

pub fn show_nav(app: &mut NaranjaHubApp, ctx: &egui::Context) {
    let t = app.t();
    egui::SidePanel::left("naranjahub_nav")
        .resizable(false)
        .default_width(216.0)
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(Margin::same(12.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(logo) = app.logo.clone() {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(
                            logo.id(),
                            Vec2::splat(40.0),
                        ))
                        .rounding(Rounding::same(9.0)),
                    );
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t.app_title)
                            .size(19.0)
                            .strong()
                            .color(theme::ORANGE),
                    );
                    ui.label(RichText::new(t.tagline).small().color(theme::MUTED));
                });
            });
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);
            let tabs = [
                (Tab::Library, t.nav_library),
                (Tab::Integrate, t.nav_integrate),
                (Tab::Updates, t.nav_updates),
                (Tab::Activity, t.nav_activity),
                (Tab::Settings, t.nav_settings),
                (Tab::About, t.nav_about),
            ];
            for (tab, label) in tabs {
                let selected = app.tab == tab;
                let mut button = if selected {
                    egui::Button::new(RichText::new(label).strong().color(theme::BG))
                        .fill(theme::ORANGE)
                } else {
                    egui::Button::new(RichText::new(label).color(theme::TEXT))
                        .fill(Color32::TRANSPARENT)
                };
                button = button
                    .rounding(Rounding::same(8.0))
                    .min_size(Vec2::new(ui.available_width(), 34.0));
                if ui.add(button).clicked() {
                    app.tab = tab;
                }
            }
            ui.with_layout(egui::Layout::bottom_up(Align::LEFT), |ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("v{} · GNU GPL v3", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(theme::MUTED),
                );
                ui.label(RichText::new(t.ab_created_by).small().color(theme::MUTED));
            });
        });
}

pub fn show_status_bar(app: &mut NaranjaHubApp, ctx: &egui::Context) {
    let t = app.t();
    let count = app.library.len().to_string();
    let dir = app.dirs.apps_dir.display().to_string();
    egui::TopBottomPanel::bottom("naranjahub_status")
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(Margin::symmetric(14.0, 6.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(utils::tf(
                        t.lib_apps_in,
                        &[("n", count.as_str()), ("dir", dir.as_str())],
                    ))
                    .small()
                    .color(theme::MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("NaranjaHub v{}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(theme::MUTED),
                    );
                });
            });
        });
}

pub fn show_toasts(app: &mut NaranjaHubApp, ctx: &egui::Context) {
    app.toasts
        .retain(|toast| toast.born.elapsed() < Duration::from_secs(4));
    if app.toasts.is_empty() {
        return;
    }
    let toasts = app.toasts.clone();
    egui::Area::new(egui::Id::new("naranjahub_toasts"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -36.0])
        .show(ctx, |ui| {
            for toast in toasts {
                let color = match toast.kind {
                    ToastKind::Ok => theme::OK,
                    ToastKind::Error => theme::DANGER,
                    ToastKind::Info => theme::ORANGE,
                };
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0_f32, color))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot.center(), 4.0, color);
                            ui.label(RichText::new(toast.msg).color(theme::TEXT));
                        });
                    });
                ui.add_space(6.0);
            }
        });
}

pub fn show_confirm_dialog(app: &mut NaranjaHubApp, ctx: &egui::Context) {
    let Some(path) = app.pending_delete.clone() else {
        return;
    };
    let t = app.t();
    egui::Area::new(egui::Id::new("naranjahub_dim"))
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::ZERO)
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(ctx.screen_rect(), 0.0, Color32::from_black_alpha(150));
        });
    egui::Area::new(egui::Id::new("naranjahub_confirm"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(14.0))
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(400.0);
                    ui.label(
                        RichText::new(t.cfm_delete_title)
                            .heading()
                            .color(theme::TEXT),
                    );
                    ui.add_space(4.0);
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    ui.label(
                        RichText::new(utils::tf(t.cfm_delete_body, &[("name", name.as_str())]))
                            .color(theme::MUTED),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.add(theme::danger_button(t.cfm_yes_delete)).clicked() {
                            app.pending_delete = None;
                            app.delete_confirmed(path.clone());
                        }
                        if ui.add(theme::ghost_button(t.cfm_cancel)).clicked() {
                            app.pending_delete = None;
                        }
                    });
                });
        });
}
