//! Modern, high-precision Color Picker component for the Settings UI.
//!
//! Provides an intuitive, Figma/macOS-grade interface:
//! - Visual trigger capsule with live swatch and formatted hex code.
//! - 2D Saturation / Value bilinear gradient canvas with crisp reticle.
//! - Spectrum Hue slider with smooth rainbow gradient.
//! - Transparency Alpha slider with checkerboard underlay and percentage display.
//! - Multi-mode channel editors (HEX, RGB 0-255, HSV 0-360°/100%).
//! - Quick-select curated palette swatches for fast theme styling.

use eframe::egui::{
    self, epaint::Vertex, Color32, Mesh, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
};

use crate::gui::theme;

/// Active input format mode inside the color panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    #[default]
    Hex,
    Rgb,
    Hsv,
}

/// Internal state stored per-color-picker instance across frames.
#[derive(Debug, Clone)]
pub struct ColorPickerState {
    pub h: f32,
    pub s: f32,
    pub v: f32,
    pub a: f32,
    pub hex_text: String,
    pub mode: ColorMode,
    pub initialized: bool,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self {
            h: 0.0,
            s: 1.0,
            v: 1.0,
            a: 1.0,
            hex_text: "#FFFFFF".to_string(),
            mode: ColorMode::Hex,
            initialized: false,
        }
    }
}

// ----------------------------------------------------------------------------
// Color Conversion Utilities
// ----------------------------------------------------------------------------

pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let v = max;
    let s = if max > 1e-5 { delta / max } else { 0.0 };

    let mut h = if delta < 1e-5 {
        0.0
    } else if (max - r).abs() < 1e-5 {
        (g - b) / delta
    } else if (max - g).abs() < 1e-5 {
        2.0 + (b - r) / delta
    } else {
        4.0 + (r - g) / delta
    };

    h /= 6.0;
    if h < 0.0 {
        h += 1.0;
    }
    (h, s, v)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    if s <= 1e-5 {
        return (v, v, v);
    }
    let h_sector = (h.fract() * 6.0).clamp(0.0, 5.99999);
    let i = h_sector.floor() as i32;
    let f = h_sector - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

pub fn rgba_to_hex(rgba: [f32; 4], include_alpha: bool) -> String {
    let r = (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8;
    if include_alpha && a < 255 {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    } else {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }
}

pub fn hex_to_rgba(hex: &str) -> Option<[f32; 4]> {
    let s = hex.trim().trim_start_matches('#');
    match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
        4 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&s[3..4].repeat(2), 16).ok()?;
            Some([
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ])
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some([
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ])
        }
        _ => None,
    }
}

// ----------------------------------------------------------------------------
// Drawing Helpers
// ----------------------------------------------------------------------------

fn draw_checkerboard(
    painter: &egui::Painter,
    rect: Rect,
    cell_size: f32,
    c1: Color32,
    c2: Color32,
) {
    let cols = (rect.width() / cell_size).ceil() as usize;
    let rows = (rect.height() / cell_size).ceil() as usize;

    for r in 0..rows {
        for c in 0..cols {
            let cell_rect = Rect::from_min_size(
                Pos2::new(
                    rect.left() + c as f32 * cell_size,
                    rect.top() + r as f32 * cell_size,
                ),
                Vec2::splat(cell_size),
            )
            .intersect(rect);

            let color = if (r + c) % 2 == 0 { c1 } else { c2 };
            painter.rect_filled(cell_rect, 0.0, color);
        }
    }
}

// ----------------------------------------------------------------------------
// Main Color Picker Component
// ----------------------------------------------------------------------------

/// Render a modern trigger capsule that opens a comprehensive color editing popup.
pub fn color_picker_button(ui: &mut Ui, id_source: &str, color: &mut [f32; 4]) -> Response {
    let id = ui.make_persistent_id(id_source);
    let popup_id = id.with("popup");

    let alpha_pct = (color[3].clamp(0.0, 1.0) * 100.0).round() as u8;
    let hex_display = rgba_to_hex(*color, false);

    // Pill trigger dimensions
    let button_size = Vec2::new(124.0, 28.0);
    let (rect, mut response) = ui.allocate_exact_size(button_size, Sense::click());

    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }

    let is_open = ui.memory(|m| m.is_popup_open(popup_id));

    if ui.is_rect_visible(rect) {
        let rounding = Rounding::same(6.0);

        // Background of the trigger button
        let bg = if is_open {
            theme::surface_active()
        } else if response.hovered() {
            theme::surface_hover()
        } else {
            theme::surface()
        };

        ui.painter().rect_filled(rect, rounding, bg);
        ui.painter().rect_stroke(
            rect,
            rounding,
            Stroke::new(
                1.0,
                if is_open {
                    theme::accent()
                } else if response.hovered() {
                    theme::divider()
                } else {
                    theme::divider().gamma_multiply(0.6)
                },
            ),
        );

        // Color preview swatch box (left side)
        let swatch_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 6.0, rect.top() + 5.0),
            Vec2::new(26.0, 18.0),
        );
        let swatch_rounding = Rounding::same(4.0);

        // Checkerboard underlay for transparency preview
        draw_checkerboard(
            ui.painter(),
            swatch_rect,
            4.0,
            Color32::from_gray(55),
            Color32::from_gray(95),
        );

        let preview_color = Color32::from_rgba_unmultiplied(
            (color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (color[3].clamp(0.0, 1.0) * 255.0) as u8,
        );
        ui.painter()
            .rect_filled(swatch_rect, swatch_rounding, preview_color);
        ui.painter().rect_stroke(
            swatch_rect,
            swatch_rounding,
            Stroke::new(1.0, Color32::from_white_alpha(35)),
        );

        // Hex Code Text
        let text_pos = Pos2::new(rect.left() + 38.0, rect.center().y);
        let font_id = egui::FontId::monospace(11.5);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            &hex_display,
            font_id,
            theme::text_primary(),
        );

        // Opacity tag or dropdown indicator (right side)
        let right_pos = Pos2::new(rect.right() - 8.0, rect.center().y);
        let tag_text = if alpha_pct < 100 {
            format!("{}%", alpha_pct)
        } else {
            "▾".to_string()
        };
        ui.painter().text(
            right_pos,
            egui::Align2::RIGHT_CENTER,
            tag_text,
            egui::FontId::proportional(10.0),
            theme::text_tertiary(),
        );
    }

    // Popup Panel Rendering
    let mut changed = false;
    egui::popup::popup_above_or_below_widget(
        ui,
        popup_id,
        &response,
        egui::AboveOrBelow::Below,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(260.0);
            ui.set_max_width(260.0);

            let state_id = id.with("state");
            let mut state: ColorPickerState =
                ui.data_mut(|d| d.get_temp(state_id)).unwrap_or_default();

            // Sync state from color on open or first init
            if !state.initialized {
                let (h, s, v) = rgb_to_hsv(color[0], color[1], color[2]);
                state.h = h;
                state.s = s;
                state.v = v;
                state.a = color[3];
                state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                state.initialized = true;
            }

            ui.add_space(2.0);

            // Top Header: Live Swatch & Active Hex Overview
            ui.horizontal(|ui| {
                let header_swatch_size = Vec2::new(32.0, 24.0);
                let (h_rect, _) = ui.allocate_exact_size(header_swatch_size, Sense::hover());
                let h_round = Rounding::same(4.0);

                draw_checkerboard(
                    ui.painter(),
                    h_rect,
                    4.0,
                    Color32::from_gray(50),
                    Color32::from_gray(90),
                );

                let c_active = Color32::from_rgba_unmultiplied(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    (color[3] * 255.0) as u8,
                );
                ui.painter().rect_filled(h_rect, h_round, c_active);
                ui.painter()
                    .rect_stroke(h_rect, h_round, Stroke::new(1.0, theme::divider()));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(rgba_to_hex(*color, state.a < 1.0))
                                .monospace()
                                .strong()
                                .size(12.0)
                                .color(theme::text_primary()),
                        );
                        if ui
                            .small_button("📋")
                            .on_hover_text("Copy Hex Code")
                            .clicked()
                        {
                            ui.output_mut(|o| o.copied_text = rgba_to_hex(*color, state.a < 1.0));
                        }
                    });
                });
            });

            ui.add_space(6.0);

            // 1. 2D Saturation / Value Canvas
            let sv_size = Vec2::new(260.0, 150.0);
            let (sv_rect, sv_resp) = ui.allocate_exact_size(sv_size, Sense::click_and_drag());

            if sv_resp.dragged() || sv_resp.clicked() {
                if let Some(pos) = sv_resp.interact_pointer_pos() {
                    let rel_x = ((pos.x - sv_rect.left()) / sv_rect.width()).clamp(0.0, 1.0);
                    let rel_y = ((pos.y - sv_rect.top()) / sv_rect.height()).clamp(0.0, 1.0);
                    state.s = rel_x;
                    state.v = 1.0 - rel_y;

                    let (r, g, b) = hsv_to_rgb(state.h, state.s, state.v);
                    color[0] = r;
                    color[1] = g;
                    color[2] = b;
                    state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                    changed = true;
                }
            }

            // Draw Bilinear Mesh for Saturation-Value
            if ui.is_rect_visible(sv_rect) {
                let mut mesh = Mesh::default();
                let pure_hue = Color32::from_rgb(
                    (hsv_to_rgb(state.h, 1.0, 1.0).0 * 255.0) as u8,
                    (hsv_to_rgb(state.h, 1.0, 1.0).1 * 255.0) as u8,
                    (hsv_to_rgb(state.h, 1.0, 1.0).2 * 255.0) as u8,
                );

                let tl = Pos2::new(sv_rect.left(), sv_rect.top());
                let tr = Pos2::new(sv_rect.right(), sv_rect.top());
                let bl = Pos2::new(sv_rect.left(), sv_rect.bottom());
                let br = Pos2::new(sv_rect.right(), sv_rect.bottom());

                mesh.vertices.push(Vertex {
                    pos: tl,
                    uv: Pos2::ZERO,
                    color: Color32::WHITE,
                });
                mesh.vertices.push(Vertex {
                    pos: tr,
                    uv: Pos2::ZERO,
                    color: pure_hue,
                });
                mesh.vertices.push(Vertex {
                    pos: br,
                    uv: Pos2::ZERO,
                    color: Color32::BLACK,
                });
                mesh.vertices.push(Vertex {
                    pos: bl,
                    uv: Pos2::ZERO,
                    color: Color32::BLACK,
                });

                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                ui.painter().add(mesh);

                ui.painter().rect_stroke(
                    sv_rect,
                    Rounding::same(4.0),
                    Stroke::new(1.0, theme::divider()),
                );

                // SV Reticle handle
                let handle_pos = Pos2::new(
                    sv_rect.left() + state.s * sv_rect.width(),
                    sv_rect.top() + (1.0 - state.v) * sv_rect.height(),
                );

                ui.painter()
                    .circle_filled(handle_pos, 7.0, Color32::from_black_alpha(80));
                ui.painter()
                    .circle_stroke(handle_pos, 6.0, Stroke::new(2.0, Color32::WHITE));
                ui.painter()
                    .circle_stroke(handle_pos, 4.0, Stroke::new(1.0, Color32::BLACK));
            }

            ui.add_space(8.0);

            // 2. Rainbow Hue Slider
            let slider_size = Vec2::new(260.0, 14.0);
            let (hue_rect, hue_resp) = ui.allocate_exact_size(slider_size, Sense::click_and_drag());

            if hue_resp.dragged() || hue_resp.clicked() {
                if let Some(pos) = hue_resp.interact_pointer_pos() {
                    state.h = ((pos.x - hue_rect.left()) / hue_rect.width()).clamp(0.0, 1.0);
                    let (r, g, b) = hsv_to_rgb(state.h, state.s, state.v);
                    color[0] = r;
                    color[1] = g;
                    color[2] = b;
                    state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                    changed = true;
                }
            }

            if ui.is_rect_visible(hue_rect) {
                let segments = 12;
                let seg_w = hue_rect.width() / segments as f32;
                for i in 0..segments {
                    let h1 = i as f32 / segments as f32;
                    let h2 = (i + 1) as f32 / segments as f32;
                    let r1 = Pos2::new(hue_rect.left() + i as f32 * seg_w, hue_rect.top());
                    let r2 = Pos2::new(hue_rect.left() + (i + 1) as f32 * seg_w, hue_rect.bottom());
                    let rect = Rect::from_min_max(r1, r2);

                    let c1 = Color32::from_rgb(
                        (hsv_to_rgb(h1, 1.0, 1.0).0 * 255.0) as u8,
                        (hsv_to_rgb(h1, 1.0, 1.0).1 * 255.0) as u8,
                        (hsv_to_rgb(h1, 1.0, 1.0).2 * 255.0) as u8,
                    );
                    let c2 = Color32::from_rgb(
                        (hsv_to_rgb(h2, 1.0, 1.0).0 * 255.0) as u8,
                        (hsv_to_rgb(h2, 1.0, 1.0).1 * 255.0) as u8,
                        (hsv_to_rgb(h2, 1.0, 1.0).2 * 255.0) as u8,
                    );

                    let mut mesh = Mesh::default();
                    mesh.vertices.push(Vertex {
                        pos: rect.left_top(),
                        uv: Pos2::ZERO,
                        color: c1,
                    });
                    mesh.vertices.push(Vertex {
                        pos: rect.right_top(),
                        uv: Pos2::ZERO,
                        color: c2,
                    });
                    mesh.vertices.push(Vertex {
                        pos: rect.right_bottom(),
                        uv: Pos2::ZERO,
                        color: c2,
                    });
                    mesh.vertices.push(Vertex {
                        pos: rect.left_bottom(),
                        uv: Pos2::ZERO,
                        color: c1,
                    });
                    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                    ui.painter().add(mesh);
                }

                ui.painter().rect_stroke(
                    hue_rect,
                    Rounding::same(6.0),
                    Stroke::new(1.0, theme::divider()),
                );

                // Hue Thumb
                let thumb_x = (hue_rect.left() + state.h * hue_rect.width())
                    .clamp(hue_rect.left() + 6.0, hue_rect.right() - 6.0);
                let thumb_pos = Pos2::new(thumb_x, hue_rect.center().y);
                ui.painter().circle_filled(thumb_pos, 7.0, Color32::WHITE);
                ui.painter().circle_stroke(
                    thumb_pos,
                    7.0,
                    Stroke::new(1.5, Color32::from_black_alpha(120)),
                );
            }

            ui.add_space(6.0);

            // 3. Alpha / Opacity Slider
            let (alpha_rect, alpha_resp) =
                ui.allocate_exact_size(slider_size, Sense::click_and_drag());

            if alpha_resp.dragged() || alpha_resp.clicked() {
                if let Some(pos) = alpha_resp.interact_pointer_pos() {
                    state.a = ((pos.x - alpha_rect.left()) / alpha_rect.width()).clamp(0.0, 1.0);
                    color[3] = state.a;
                    state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                    changed = true;
                }
            }

            if ui.is_rect_visible(alpha_rect) {
                draw_checkerboard(
                    ui.painter(),
                    alpha_rect,
                    4.0,
                    Color32::from_gray(50),
                    Color32::from_gray(90),
                );

                let c_opaque = Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                let c_transparent = Color32::from_rgba_unmultiplied(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    0,
                );

                let mut mesh = Mesh::default();
                mesh.vertices.push(Vertex {
                    pos: alpha_rect.left_top(),
                    uv: Pos2::ZERO,
                    color: c_transparent,
                });
                mesh.vertices.push(Vertex {
                    pos: alpha_rect.right_top(),
                    uv: Pos2::ZERO,
                    color: c_opaque,
                });
                mesh.vertices.push(Vertex {
                    pos: alpha_rect.right_bottom(),
                    uv: Pos2::ZERO,
                    color: c_opaque,
                });
                mesh.vertices.push(Vertex {
                    pos: alpha_rect.left_bottom(),
                    uv: Pos2::ZERO,
                    color: c_transparent,
                });
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                ui.painter().add(mesh);

                ui.painter().rect_stroke(
                    alpha_rect,
                    Rounding::same(6.0),
                    Stroke::new(1.0, theme::divider()),
                );

                // Alpha Thumb
                let thumb_x = (alpha_rect.left() + state.a * alpha_rect.width())
                    .clamp(alpha_rect.left() + 6.0, alpha_rect.right() - 6.0);
                let thumb_pos = Pos2::new(thumb_x, alpha_rect.center().y);
                ui.painter().circle_filled(thumb_pos, 7.0, Color32::WHITE);
                ui.painter().circle_stroke(
                    thumb_pos,
                    7.0,
                    Stroke::new(1.5, Color32::from_black_alpha(120)),
                );
            }

            ui.add_space(8.0);

            // 4. Mode Switcher (HEX / RGB / HSV) & Numeric Inputs
            ui.horizontal(|ui| {
                for (mode, label) in [
                    (ColorMode::Hex, "HEX"),
                    (ColorMode::Rgb, "RGB"),
                    (ColorMode::Hsv, "HSV"),
                ] {
                    let selected = state.mode == mode;
                    if ui.selectable_label(selected, label).clicked() && !selected {
                        state.mode = mode;
                    }
                }
            });

            ui.add_space(4.0);

            match state.mode {
                ColorMode::Hex => {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("#")
                                .monospace()
                                .color(theme::text_tertiary()),
                        );
                        let hex_input = ui.add_sized(
                            Vec2::new(130.0, 22.0),
                            egui::TextEdit::singleline(&mut state.hex_text)
                                .font(egui::FontId::monospace(12.0))
                                .hint_text("RRGGBB"),
                        );

                        if hex_input.lost_focus() || hex_input.changed() {
                            if let Some(parsed) = hex_to_rgba(&state.hex_text) {
                                color[0] = parsed[0];
                                color[1] = parsed[1];
                                color[2] = parsed[2];
                                if state.hex_text.len() > 7 {
                                    color[3] = parsed[3];
                                    state.a = parsed[3];
                                }
                                let (h, s, v) = rgb_to_hsv(color[0], color[1], color[2]);
                                state.h = h;
                                state.s = s;
                                state.v = v;
                                changed = true;
                            }
                        }

                        // Opacity percentage
                        let mut pct = (state.a * 100.0).round() as i32;
                        if ui
                            .add_sized(
                                Vec2::new(60.0, 22.0),
                                egui::DragValue::new(&mut pct).range(0..=100).suffix("%"),
                            )
                            .changed()
                        {
                            state.a = (pct as f32 / 100.0).clamp(0.0, 1.0);
                            color[3] = state.a;
                            state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                            changed = true;
                        }
                    });
                }
                ColorMode::Rgb => {
                    ui.horizontal(|ui| {
                        let mut r = (color[0] * 255.0).round() as u8;
                        let mut g = (color[1] * 255.0).round() as u8;
                        let mut b = (color[2] * 255.0).round() as u8;
                        let mut a = (color[3] * 255.0).round() as u8;

                        ui.spacing_mut().item_spacing.x = 4.0;
                        if ui
                            .add(egui::DragValue::new(&mut r).prefix("R:").range(0..=255))
                            .changed()
                            || ui
                                .add(egui::DragValue::new(&mut g).prefix("G:").range(0..=255))
                                .changed()
                            || ui
                                .add(egui::DragValue::new(&mut b).prefix("B:").range(0..=255))
                                .changed()
                            || ui
                                .add(egui::DragValue::new(&mut a).prefix("A:").range(0..=255))
                                .changed()
                        {
                            color[0] = r as f32 / 255.0;
                            color[1] = g as f32 / 255.0;
                            color[2] = b as f32 / 255.0;
                            color[3] = a as f32 / 255.0;
                            state.a = color[3];
                            let (h, s, v) = rgb_to_hsv(color[0], color[1], color[2]);
                            state.h = h;
                            state.s = s;
                            state.v = v;
                            state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                            changed = true;
                        }
                    });
                }
                ColorMode::Hsv => {
                    ui.horizontal(|ui| {
                        let mut h_deg = (state.h * 360.0).round() as i32;
                        let mut s_pct = (state.s * 100.0).round() as i32;
                        let mut v_pct = (state.v * 100.0).round() as i32;
                        let mut a_pct = (state.a * 100.0).round() as i32;

                        ui.spacing_mut().item_spacing.x = 4.0;
                        if ui
                            .add(
                                egui::DragValue::new(&mut h_deg)
                                    .prefix("H:")
                                    .range(0..=360)
                                    .suffix("°"),
                            )
                            .changed()
                            || ui
                                .add(
                                    egui::DragValue::new(&mut s_pct)
                                        .prefix("S:")
                                        .range(0..=100)
                                        .suffix("%"),
                                )
                                .changed()
                            || ui
                                .add(
                                    egui::DragValue::new(&mut v_pct)
                                        .prefix("V:")
                                        .range(0..=100)
                                        .suffix("%"),
                                )
                                .changed()
                            || ui
                                .add(
                                    egui::DragValue::new(&mut a_pct)
                                        .prefix("A:")
                                        .range(0..=100)
                                        .suffix("%"),
                                )
                                .changed()
                        {
                            state.h = (h_deg as f32 / 360.0).clamp(0.0, 1.0);
                            state.s = (s_pct as f32 / 100.0).clamp(0.0, 1.0);
                            state.v = (v_pct as f32 / 100.0).clamp(0.0, 1.0);
                            state.a = (a_pct as f32 / 100.0).clamp(0.0, 1.0);

                            let (r, g, b) = hsv_to_rgb(state.h, state.s, state.v);
                            color[0] = r;
                            color[1] = g;
                            color[2] = b;
                            color[3] = state.a;
                            state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                            changed = true;
                        }
                    });
                }
            }

            ui.add_space(8.0);

            // 5. Curated Presets Swatches Grid
            ui.label(
                egui::RichText::new("PRESETS")
                    .size(10.0)
                    .color(theme::text_tertiary()),
            );
            ui.add_space(2.0);

            let presets: &[[f32; 4]] = &[
                // Neutrals & Dark
                [0.031, 0.031, 0.043, 1.0], // Obsidian
                [0.10, 0.12, 0.16, 1.0],    // Slate
                [0.40, 0.42, 0.48, 1.0],    // Steel Grey
                [0.85, 0.86, 0.90, 1.0],    // Mist
                [0.98, 0.98, 1.00, 1.0],    // Cloud White
                [0.07, 0.07, 0.09, 0.55],   // Frosted Tint
                // Vibrant Accents
                [0.43, 0.55, 0.98, 1.0], // Accent Blue
                [0.23, 0.51, 0.96, 1.0], // Electric Blue
                [0.02, 0.71, 0.83, 1.0], // Cyan
                [0.06, 0.73, 0.51, 1.0], // Emerald
                [0.96, 0.62, 0.04, 1.0], // Amber
                [0.98, 0.45, 0.09, 1.0], // Orange
                [0.96, 0.25, 0.37, 1.0], // Coral Red
                [0.55, 0.36, 0.96, 1.0], // Violet
            ];

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(4.0);
                for swatch in presets {
                    let swatch_size = Vec2::splat(16.0);
                    let (s_rect, s_resp) = ui.allocate_exact_size(swatch_size, Sense::click());

                    let c = Color32::from_rgba_unmultiplied(
                        (swatch[0] * 255.0) as u8,
                        (swatch[1] * 255.0) as u8,
                        (swatch[2] * 255.0) as u8,
                        (swatch[3] * 255.0) as u8,
                    );

                    if s_resp.clicked() {
                        *color = *swatch;
                        let (h, s, v) = rgb_to_hsv(color[0], color[1], color[2]);
                        state.h = h;
                        state.s = s;
                        state.v = v;
                        state.a = color[3];
                        state.hex_text = rgba_to_hex(*color, state.a < 1.0);
                        changed = true;
                    }

                    if ui.is_rect_visible(s_rect) {
                        draw_checkerboard(
                            ui.painter(),
                            s_rect,
                            3.0,
                            Color32::from_gray(50),
                            Color32::from_gray(90),
                        );
                        let s_rounding = Rounding::same(3.0);
                        ui.painter().rect_filled(s_rect, s_rounding, c);
                        ui.painter().rect_stroke(
                            s_rect,
                            s_rounding,
                            Stroke::new(
                                if s_resp.hovered() { 1.5 } else { 0.5 },
                                if s_resp.hovered() {
                                    theme::text_primary()
                                } else {
                                    theme::divider()
                                },
                            ),
                        );
                    }
                }
            });

            ui.add_space(2.0);

            // Persist modified picker state
            ui.data_mut(|d| d.insert_temp(state_id, state));
        },
    );

    if changed {
        response.mark_changed();
    }

    response
}
