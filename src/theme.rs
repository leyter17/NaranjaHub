// NaranjaHub — tema visual: naranja sobre negro, esquinas suaves.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{self, Color32, FontId, Rounding, Stroke, TextStyle, Visuals};

pub const BG: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0D);
pub const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x17);
pub const CARD: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x20);
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x22, 0x22, 0x28);
pub const BORDER: Color32 = Color32::from_rgb(0x2C, 0x2C, 0x33);
pub const ORANGE: Color32 = Color32::from_rgb(0xFF, 0x7A, 0x1A);
pub const ORANGE_HOVER: Color32 = Color32::from_rgb(0xFF, 0x92, 0x3D);
pub const ORANGE_PRESSED: Color32 = Color32::from_rgb(0xE0, 0x69, 0x0F);
pub const TEXT: Color32 = Color32::from_rgb(0xF0, 0xEE, 0xE9);
pub const MUTED: Color32 = Color32::from_rgb(0xA8, 0xA4, 0x9C);
pub const OK: Color32 = Color32::from_rgb(0x6F, 0xC8, 0x7B);
pub const DANGER: Color32 = Color32::from_rgb(0xE5, 0x53, 0x4B);
pub const WARN: Color32 = Color32::from_rgb(0xF0, 0xB4, 0x29);

pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut style = (*ctx.style()).clone();
    style.visuals = dark_visuals();
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(21.0)),
        (TextStyle::Body, FontId::proportional(15.0)),
        (TextStyle::Button, FontId::proportional(15.0)),
        (TextStyle::Small, FontId::proportional(12.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.bar_inner_margin = 3.0;
    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = CARD;
    v.extreme_bg_color = BG;
    v.faint_bg_color = CARD;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);
    v.hyperlink_color = ORANGE;
    v.selection.bg_fill = ORANGE;
    v.selection.stroke = Stroke::new(1.0_f32, BG);
    v.widgets.noninteractive.bg_fill = CARD;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.hovered.bg_fill = CARD_HOVER;
    v.widgets.hovered.weak_bg_fill = CARD_HOVER;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, ORANGE);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ORANGE);
    v.widgets.active.bg_fill = ORANGE_PRESSED;
    v.widgets.active.weak_bg_fill = ORANGE_PRESSED;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.open.bg_fill = CARD_HOVER;
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32, ORANGE);
    v
}

pub fn primary_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).strong().color(BG))
        .fill(ORANGE)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(8.0))
}

pub fn secondary_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).color(ORANGE))
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, ORANGE))
        .rounding(Rounding::same(8.0))
}

pub fn danger_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).strong().color(TEXT))
        .fill(DANGER)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(8.0))
}

pub fn ghost_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).color(TEXT))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(8.0))
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).heading().strong().color(TEXT));
}
