use eframe::egui::{
    self, Align, Align2, Color32, Frame, Layout, Margin, Rect, RichText, Rounding, Sense, Stroke,
    Vec2,
};
use parking_lot::RwLock;
use std::sync::Arc;

use crate::config::AppConfig;

mod colors {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(10, 10, 13);
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(14, 14, 18);
    pub const SURFACE: Color32 = Color32::from_rgb(20, 20, 25);
    pub const SURFACE_HOVER: Color32 = Color32::from_rgb(28, 28, 34);
    pub const DIVIDER: Color32 = Color32::from_rgb(38, 38, 45);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 243);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(148, 148, 158);
    pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(100, 100, 110);
    pub const ACCENT: Color32 = Color32::from_rgb(99, 132, 245);
}

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "PlusJakartaSans".to_owned(),
        egui::FontData::from_static(include_bytes!("../../PlusJakartaSans.ttf")),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "PlusJakartaSans".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "PlusJakartaSans".to_owned());

    ctx.set_fonts(fonts);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Text,
    Layout,
    Appearance,
    Behavior,
}

pub struct SettingsApp {
    config: Arc<RwLock<AppConfig>>,
    temp_text: String,
    active_tab: Tab,
}

impl SettingsApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Arc<RwLock<AppConfig>>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.window_rounding = Rounding::same(10.0);
        style.visuals.window_fill = colors::BG;
        style.visuals.panel_fill = colors::BG;

        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 6.0);

        for widget_style in [
            &mut style.visuals.widgets.noninteractive,
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget_style.rounding = Rounding::same(6.0);
        }

        style.visuals.widgets.noninteractive.bg_fill = colors::SURFACE;
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors::DIVIDER);
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);

        style.visuals.widgets.inactive.bg_fill = colors::SURFACE;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors::DIVIDER);
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);

        style.visuals.widgets.hovered.bg_fill = colors::SURFACE_HOVER;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors::ACCENT);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors::TEXT_PRIMARY);

        style.visuals.widgets.active.bg_fill = colors::ACCENT;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors::ACCENT);
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors::BG);

        style.visuals.selection.bg_fill = colors::ACCENT;
        style.visuals.selection.stroke = Stroke::new(1.0, colors::ACCENT);

        cc.egui_ctx.set_style(style);

        let current_text = config.read().text.clone();
        Self {
            config,
            temp_text: current_text,
            active_tab: Tab::Text,
        }
    }

    fn section_title(ui: &mut egui::Ui, title: &str) {
        ui.label(
            RichText::new(title)
                .size(11.0)
                .color(colors::TEXT_TERTIARY)
                .strong(),
        );
        ui.add_space(10.0);
    }

    fn divider(ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(14.0);
    }

    fn row_stacked(
        ui: &mut egui::Ui,
        label: &str,
        help: Option<&str>,
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        ui.label(RichText::new(label).size(13.0).color(colors::TEXT_PRIMARY));
        if let Some(h) = help {
            ui.add_space(2.0);
            ui.label(RichText::new(h).size(11.0).color(colors::TEXT_SECONDARY));
        }
        ui.add_space(8.0);
        add_contents(ui);
        ui.add_space(18.0);
    }

    fn row_inline(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(13.0).color(colors::TEXT_PRIMARY));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                add_contents(ui);
            });
        });
        ui.add_space(12.0);
    }

    fn draw_nav(ui: &mut egui::Ui, active_tab: &mut Tab) {
        ui.add_space(6.0);
        ui.label(
            RichText::new("SECTIONS")
                .size(10.0)
                .color(colors::TEXT_TERTIARY)
                .strong(),
        );
        ui.add_space(8.0);

        let tabs = [
            (Tab::Text, "Text"),
            (Tab::Layout, "Layout"),
            (Tab::Appearance, "Appearance"),
            (Tab::Behavior, "Behavior"),
        ];

        for (tab, label) in tabs {
            let selected = *active_tab == tab;
            let width = ui.available_width();
            let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());

            if selected {
                ui.painter()
                    .rect_filled(rect, Rounding::same(6.0), colors::SURFACE);
                let bar = Rect::from_min_size(rect.min, Vec2::new(2.5, rect.height()));
                ui.painter()
                    .rect_filled(bar, Rounding::same(2.0), colors::ACCENT);
            } else if response.hovered() {
                ui.painter()
                    .rect_filled(rect, Rounding::same(6.0), colors::SURFACE_HOVER);
            }

            let text_color = if selected {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            };

            ui.painter().text(
                rect.left_center() + Vec2::new(14.0, 0.0),
                Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                text_color,
            );

            if response.clicked() {
                *active_tab = tab;
            }

            ui.add_space(2.0);
        }
    }

    fn draw_preview_bar(ui: &mut egui::Ui, cfg: &AppConfig) {
        ui.horizontal(|ui| {
            let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 3.0, colors::ACCENT);

            ui.label(
                RichText::new("PREVIEW")
                    .size(10.0)
                    .color(colors::TEXT_TERTIARY)
                    .strong(),
            );
            ui.add_space(14.0);

            let bg_color = Color32::from_rgba_unmultiplied(
                (cfg.colors.bg_color[0] * 255.0) as u8,
                (cfg.colors.bg_color[1] * 255.0) as u8,
                (cfg.colors.bg_color[2] * 255.0) as u8,
                (cfg.colors.bg_color[3] * 255.0) as u8,
            );
            let fg_color = Color32::from_rgba_unmultiplied(
                (cfg.colors.text_color[0] * 255.0) as u8,
                (cfg.colors.text_color[1] * 255.0) as u8,
                (cfg.colors.text_color[2] * 255.0) as u8,
                (cfg.colors.text_color[3] * 255.0) as u8,
            );

            Frame::default()
                .fill(bg_color)
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(14.0, 6.0))
                .show(ui, |ui| {
                    ui.set_clip_rect(ui.max_rect());
                    let spacing_str = " ".repeat(cfg.phrase_spacing as usize);
                    let display_text =
                        format!("{}{}{}{}", cfg.text, spacing_str, cfg.text, spacing_str);

                    let mut text_rt = RichText::new(&display_text)
                        .size(cfg.font.size.clamp(13.0, 22.0))
                        .color(fg_color);
                    if cfg.font.bold {
                        text_rt = text_rt.strong();
                    }
                    if cfg.font.italic {
                        text_rt = text_rt.italics();
                    }
                    ui.label(text_rt);
                });
        });
    }

    fn tab_text(
        ui: &mut egui::Ui,
        temp_text: &mut String,
        cfg: &mut AppConfig,
        changed: &mut bool,
    ) {
        Self::section_title(ui, "MARQUEE TEXT");

        Self::row_stacked(
            ui,
            "Message",
            Some("Unicode, Devanagari/Hindi, CJK and emoji are supported."),
            |ui| {
                if ui
                    .add(
                        egui::TextEdit::multiline(temp_text)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY),
                    )
                    .changed()
                {
                    cfg.text = temp_text.clone();
                    *changed = true;
                }
            },
        );

        ui.label(
            RichText::new("Presets")
                .size(12.0)
                .color(colors::TEXT_SECONDARY),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Hydration Reminder").clicked() {
                *temp_text =
                    "💧 REMINDER: Drink water & stay hydrated! • Take a 5-min break 🌿".to_string();
                cfg.text = temp_text.clone();
                *changed = true;
            }
            if ui.button("Focus Mode").clicked() {
                *temp_text = "🚀 STAY FOCUSED • Deep Work Active • Finish Tasks 🏆".to_string();
                cfg.text = temp_text.clone();
                *changed = true;
            }
            if ui.button("Hindi / CJK Sample").clicked() {
                *temp_text = "हरि प\u{941}र\u{941}ष जगद\u{94d}बन\u{94d}ध\u{941} महाउद\u{94d}धरण • 欢迎光临 • 双手合十 🌸"
                    .to_string();
                cfg.text = temp_text.clone();
                *changed = true;
            }
        });

        ui.add_space(18.0);
        Self::divider(ui);

        Self::section_title(ui, "SPACING");
        Self::row_stacked(
            ui,
            "Repetition Spacing",
            Some("Space between repeated copies of the phrase."),
            |ui| {
                if ui
                    .add(egui::Slider::new(&mut cfg.phrase_spacing, 1..=50).text("spaces"))
                    .changed()
                {
                    *changed = true;
                }
            },
        );
    }

    fn tab_layout(ui: &mut egui::Ui, cfg: &mut AppConfig, changed: &mut bool) {
        Self::section_title(ui, "ACTIVE EDGES");
        egui::Grid::new("edge_grid")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                if ui.checkbox(&mut cfg.edges.top, "Top").changed() {
                    *changed = true;
                }
                if ui.checkbox(&mut cfg.edges.right, "Right").changed() {
                    *changed = true;
                }
                ui.end_row();

                if ui.checkbox(&mut cfg.edges.bottom, "Bottom").changed() {
                    *changed = true;
                }
                if ui.checkbox(&mut cfg.edges.left, "Left").changed() {
                    *changed = true;
                }
                ui.end_row();
            });

        ui.add_space(16.0);
        Self::row_stacked(ui, "Strip Thickness", None, |ui| {
            if ui
                .add(egui::Slider::new(&mut cfg.thickness, 16..=100).text("px"))
                .changed()
            {
                *changed = true;
            }
        });

        Self::divider(ui);

        Self::section_title(ui, "CLEARANCE MARGINS");
        ui.label(
            RichText::new("Padding to avoid overlapping window controls or the taskbar.")
                .size(11.0)
                .color(colors::TEXT_SECONDARY),
        );
        ui.add_space(10.0);

        egui::Grid::new("padding_grid")
            .num_columns(4)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Top")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                );
                if ui
                    .add(egui::DragValue::new(&mut cfg.padding.top).range(0..=800))
                    .changed()
                {
                    *changed = true;
                }
                ui.label(
                    RichText::new("Right")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                );
                if ui
                    .add(egui::DragValue::new(&mut cfg.padding.right).range(0..=800))
                    .changed()
                {
                    *changed = true;
                }
                ui.end_row();

                ui.label(
                    RichText::new("Bottom")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                );
                if ui
                    .add(egui::DragValue::new(&mut cfg.padding.bottom).range(0..=800))
                    .changed()
                {
                    *changed = true;
                }
                ui.label(
                    RichText::new("Left")
                        .size(12.0)
                        .color(colors::TEXT_SECONDARY),
                );
                if ui
                    .add(egui::DragValue::new(&mut cfg.padding.left).range(0..=800))
                    .changed()
                {
                    *changed = true;
                }
                ui.end_row();
            });
    }

    fn tab_appearance(ui: &mut egui::Ui, cfg: &mut AppConfig, changed: &mut bool) {
        Self::section_title(ui, "TYPOGRAPHY");

        Self::row_inline(ui, "Font Family", |ui| {
            let fonts = [
                "Segoe UI",
                "Plus Jakarta Sans",
                "Arial",
                "Consolas",
                "Impact",
                "Microsoft YaHei",
                "Yu Gothic",
            ];
            egui::ComboBox::from_id_salt("font_family_combo")
                .selected_text(cfg.font.family.as_str())
                .show_ui(ui, |ui| {
                    for f in fonts {
                        if ui
                            .selectable_value(&mut cfg.font.family, f.to_string(), f)
                            .clicked()
                        {
                            *changed = true;
                        }
                    }
                });
        });

        Self::row_stacked(ui, "Font Size", None, |ui| {
            if ui
                .add(egui::Slider::new(&mut cfg.font.size, 10.0..=70.0).text("pt"))
                .changed()
            {
                *changed = true;
            }
        });

        ui.horizontal(|ui| {
            if ui.checkbox(&mut cfg.font.bold, "Bold").changed() {
                *changed = true;
            }
            ui.add_space(16.0);
            if ui.checkbox(&mut cfg.font.italic, "Italic").changed() {
                *changed = true;
            }
        });

        ui.add_space(6.0);
        Self::divider(ui);

        Self::section_title(ui, "COLOR");

        Self::row_inline(ui, "Text Color", |ui| {
            let mut fg = Color32::from_rgba_unmultiplied(
                (cfg.colors.text_color[0] * 255.0) as u8,
                (cfg.colors.text_color[1] * 255.0) as u8,
                (cfg.colors.text_color[2] * 255.0) as u8,
                (cfg.colors.text_color[3] * 255.0) as u8,
            );
            if ui.color_edit_button_srgba(&mut fg).changed() {
                cfg.colors.text_color = [
                    fg.r() as f32 / 255.0,
                    fg.g() as f32 / 255.0,
                    fg.b() as f32 / 255.0,
                    fg.a() as f32 / 255.0,
                ];
                *changed = true;
            }
        });

        Self::row_inline(ui, "Background Color", |ui| {
            let mut bg = Color32::from_rgba_unmultiplied(
                (cfg.colors.bg_color[0] * 255.0) as u8,
                (cfg.colors.bg_color[1] * 255.0) as u8,
                (cfg.colors.bg_color[2] * 255.0) as u8,
                (cfg.colors.bg_color[3] * 255.0) as u8,
            );
            if ui.color_edit_button_srgba(&mut bg).changed() {
                cfg.colors.bg_color = [
                    bg.r() as f32 / 255.0,
                    bg.g() as f32 / 255.0,
                    bg.b() as f32 / 255.0,
                    bg.a() as f32 / 255.0,
                ];
                *changed = true;
            }
        });
    }

    fn tab_behavior(ui: &mut egui::Ui, cfg: &mut AppConfig, changed: &mut bool) {
        Self::section_title(ui, "MOTION");

        Self::row_stacked(ui, "Scroll Speed", None, |ui| {
            if ui
                .add(egui::Slider::new(&mut cfg.animation.speed, 5.0..=500.0).text("px/s"))
                .changed()
            {
                *changed = true;
            }
        });

        Self::row_inline(ui, "Reverse Direction", |ui| {
            if ui.checkbox(&mut cfg.animation.reverse, "").changed() {
                *changed = true;
            }
        });

        Self::divider(ui);

        Self::section_title(ui, "OVERLAY BEHAVIOR");

        Self::row_inline(ui, "Always On Top", |ui| {
            if ui
                .checkbox(&mut cfg.always_on_top, "")
                .on_hover_text("Keep the text border above all open windows")
                .changed()
            {
                *changed = true;
            }
        });

        Self::row_inline(ui, "Click-Through Mode", |ui| {
            if ui
                .checkbox(&mut cfg.click_through, "")
                .on_hover_text("Allow mouse clicks to pass through the text border")
                .changed()
            {
                *changed = true;
            }
        });
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        if crate::tray::SHOW_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        }

        let mut config_changed = false;
        let mut cfg_guard = self.config.write();

        egui::TopBottomPanel::top("header")
            .frame(
                Frame::default()
                    .fill(colors::BG)
                    .inner_margin(Margin::symmetric(24.0, 14.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("MovingText")
                                .size(16.0)
                                .color(colors::TEXT_PRIMARY)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Your reminders, always in motion")
                                .size(11.0)
                                .color(colors::TEXT_SECONDARY),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Hide to Tray").size(12.0))
                            .on_hover_text("Hide the settings window to the system tray")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });

                let rect = ui.max_rect();
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    Stroke::new(1.0, colors::DIVIDER),
                );
            });

        egui::TopBottomPanel::bottom("preview")
            .exact_height(52.0)
            .frame(
                Frame::default()
                    .fill(colors::BG)
                    .inner_margin(Margin::symmetric(24.0, 8.0)),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().hline(
                    rect.x_range(),
                    rect.top(),
                    Stroke::new(1.0, colors::DIVIDER),
                );
                ui.add_space(4.0);
                Self::draw_preview_bar(ui, &cfg_guard);
            });

        egui::SidePanel::left("nav")
            .exact_width(164.0)
            .resizable(false)
            .frame(
                Frame::default()
                    .fill(colors::SIDEBAR_BG)
                    .inner_margin(Margin::symmetric(10.0, 18.0)),
            )
            .show(ctx, |ui| {
                Self::draw_nav(ui, &mut self.active_tab);
            });

        egui::CentralPanel::default()
            .frame(
                Frame::default()
                    .fill(colors::BG)
                    .inner_margin(Margin::symmetric(28.0, 22.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                    Tab::Text => {
                        Self::tab_text(ui, &mut self.temp_text, &mut cfg_guard, &mut config_changed)
                    }
                    Tab::Layout => Self::tab_layout(ui, &mut cfg_guard, &mut config_changed),
                    Tab::Appearance => {
                        Self::tab_appearance(ui, &mut cfg_guard, &mut config_changed)
                    }
                    Tab::Behavior => Self::tab_behavior(ui, &mut cfg_guard, &mut config_changed),
                });
            });

        if config_changed {
            cfg_guard.save();
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}
