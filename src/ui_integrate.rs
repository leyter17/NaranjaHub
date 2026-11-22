// NaranjaHub — pestaña Integrar.
// Copyright (C) 2026 Leyter Vázquez. V — SPDX-License-Identifier: GPL-3.0-only

use eframe::egui::{self, Align, Align2, FontId, Margin, RichText, ScrollArea, Stroke, Vec2};

use crate::app::{JobKind, JobStatus, NaranjaHubApp};
use crate::core::activity::CAT_INTEGRATE;
use crate::core::utils;
use crate::theme;

impl NaranjaHubApp {
    pub fn show_integrate(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Con scroll la pestaña nunca tapa el botón «Integrar», aunque la
        // ventana sea pequeña o el explorador esté abierto.
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.integrate_content(ui, ctx);
            });
    }

    fn integrate_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let t = self.t();
        theme::heading(ui, t.int_title);
        ui.label(RichText::new(t.int_desc).color(theme::MUTED).small());
        ui.add_space(8.0);

        // Zona de arrastrar y soltar
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let (rect, _) =
            ui.allocate_at_least(Vec2::new(ui.available_width(), 88.0), egui::Sense::hover());
        let fill = if hovering {
            theme::CARD_HOVER
        } else {
            theme::CARD
        };
        let stroke = if hovering {
            Stroke::new(2.0_f32, theme::ORANGE)
        } else {
            Stroke::new(1.0_f32, theme::BORDER)
        };
        ui.painter().rect(rect, 12.0, fill, stroke);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            t.int_hint_drop,
            FontId::proportional(15.0),
            if hovering {
                theme::ORANGE
            } else {
                theme::MUTED
            },
        );
        ui.add_space(10.0);

        // Opciones en su propia fila (con salto de línea si la ventana es
        // estrecha), separadas del botón del explorador para que NUNCA se
        // monten unas encima de otras.
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.create_entry, t.int_opt_entry);
            ui.checkbox(&mut self.move_original, t.int_opt_move);
        });
        ui.add_space(2.0);
        // El botón del explorador va en su propia línea, alineado a la
        // derecha: queda siempre visible y sin tocar la opción de mover.
        // Dentro de un «horizontal» para que la fila solo ocupe la altura
        // del botón (un right_to_left suelto se centraría en vertical en
        // todo el alto restante de la pestaña).
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let label = if self.show_browser {
                    t.int_btn_hide
                } else {
                    t.int_btn_explore
                };
                if ui.add(theme::ghost_button(label)).clicked() {
                    self.show_browser = !self.show_browser;
                }
            });
        });
        ui.add_space(4.0);

        // Integrar por ruta pegada (útil en Wayland, donde algunos
        // escritorios no entregan los archivos arrastrados).
        egui::Frame::none()
            .fill(theme::BG)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.paste_path)
                            .hint_text(t.int_paste_hint)
                            .desired_width((ui.available_width() - 130.0).max(140.0)),
                    );
                    if ui.add(theme::secondary_button(t.int_btn_paste)).clicked() {
                        let text = std::mem::take(&mut self.paste_path);
                        self.integrate_from_text(&text);
                    }
                });
                if self.backend == "wayland" {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(t.int_wayland_hint)
                            .small()
                            .color(theme::MUTED),
                    );
                }
            });
        ui.add_space(10.0);

        // Acción de integración: SIEMPRE encima del explorador para que el
        // botón «Integrar» quede visible sin necesidad de hacer scroll. Con
        // «Desmarcar» se puede descartar el archivo elegido (también con el
        // explorador cerrado).
        let selected = self.integrate_browser.selected.clone();
        egui::Frame::none()
            .fill(theme::CARD)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                // Con salto de línea: si el nombre del archivo es larguísimo
                // y la ventana estrecha, los botones pasan a la línea de
                // abajo en lugar de quedarse fuera de la pantalla.
                ui.horizontal_wrapped(|ui| match selected {
                    Some(path) => {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(
                            RichText::new(utils::tf(t.int_selected, &[("name", name.as_str())]))
                                .color(theme::TEXT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(theme::primary_button(t.int_btn_integrate)).clicked() {
                                self.integrate_path(path.clone());
                                self.integrate_browser.selected = None;
                            }
                            if ui.add(theme::ghost_button(t.int_btn_unmark)).clicked() {
                                self.integrate_browser.selected = None;
                                self.log_info(
                                    CAT_INTEGRATE,
                                    utils::tf(t.log_int_unmarked, &[("name", name.as_str())]),
                                );
                            }
                        });
                    }
                    None => {
                        ui.label(RichText::new(t.int_selected_none).color(theme::MUTED));
                    }
                });
            });
        ui.add_space(10.0);

        if self.show_browser {
            egui::Frame::none()
                .fill(theme::BG)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    self.integrate_browser.render(ui, t, false);
                });
            ui.add_space(10.0);
        }

        // Trabajos de integración en curso
        let active: Vec<(String, Option<f32>)> = self
            .jobs
            .values()
            .filter(|job| job.kind == JobKind::Integrate)
            .filter_map(|job| {
                let status = job.status();
                match status {
                    JobStatus::Working(msg) => Some((msg, None)),
                    JobStatus::Progress { got, total, .. } => {
                        let frac = total.and_then(|tt| {
                            if tt > 0 {
                                Some(got as f32 / tt as f32)
                            } else {
                                None
                            }
                        });
                        let pct = total
                            .map(|tt| {
                                let percent = if tt > 0 {
                                    100.0 * got as f32 / tt as f32
                                } else {
                                    0.0
                                };
                                format!("{percent:.0} %")
                            })
                            .unwrap_or_default();
                        Some((format!("{} {pct}", t.int_copying), frac))
                    }
                    _ => None,
                }
            })
            .collect();
        for (label, frac) in active {
            ui.add_space(6.0);
            ui.label(RichText::new(label).small().color(theme::ORANGE));
            if let Some(frac) = frac {
                ui.add(
                    egui::ProgressBar::new(frac.clamp(0.0, 1.0))
                        .desired_height(6.0)
                        .fill(theme::ORANGE_HOVER),
                );
            }
        }
    }
}
