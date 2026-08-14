//! The settings window's palette, and the machinery that lets it change.
//!
//! Every colour in the settings UI comes from here rather than from a constant
//! at the call site, which is what makes light mode possible at all: swap the
//! [`Palette`] and the whole window follows. The accessors are free functions
//! reading a thread-local, so a call site stays as short as a constant was.
//!
//! Because a `Palette` is nothing but `Color32`s, two of them can be blended.
//! That is deliberate — switching theme cross-fades over a fifth of a second
//! instead of snapping, which is the difference between the window changing
//! and the window looking like it broke.

use std::cell::Cell;

use eframe::egui::{self, Color32, Rounding, Stroke};

use crate::config::UiTheme;

/// Every tone the settings window is allowed to use.
///
/// Kept flat and `Copy` on purpose: it is read dozens of times per frame and
/// interpolated once, so an allocation anywhere in here would be a mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// True for the dark end of the blend. Only used to tell egui which set of
    /// built-in defaults to start from.
    pub dark: bool,
    /// The content area behind the pages.
    pub bg: Color32,
    /// The navigation rail. A shade apart from `bg` so the two read as
    /// separate planes rather than one wide field.
    pub sidebar: Color32,
    /// Controls: fields, buttons, cards.
    pub surface: Color32,
    /// The same, under the cursor.
    pub surface_hover: Color32,
    /// The same, pressed.
    pub surface_active: Color32,
    /// Hairlines and control outlines.
    pub divider: Color32,
    /// Body copy and control labels.
    pub text_primary: Color32,
    /// Supporting copy.
    pub text_secondary: Color32,
    /// Section headings and hints — the quietest legible tone.
    pub text_tertiary: Color32,
    /// The one saturated colour in the window.
    pub accent: Color32,
    /// A wash of the accent, for the selected navigation row.
    pub accent_wash: Color32,
    /// Type that sits on top of a filled accent.
    pub on_accent: Color32,
}

/// Near-black, with a blue-leaning grey ladder so the greys never look muddy
/// next to the accent.
pub const DARK: Palette = Palette {
    dark: true,
    bg: Color32::from_rgb(16, 16, 20),
    sidebar: Color32::from_rgb(10, 10, 13),
    surface: Color32::from_rgb(24, 24, 30),
    surface_hover: Color32::from_rgb(32, 32, 39),
    surface_active: Color32::from_rgb(40, 40, 48),
    divider: Color32::from_rgb(40, 40, 48),
    text_primary: Color32::from_rgb(240, 240, 243),
    text_secondary: Color32::from_rgb(150, 150, 160),
    text_tertiary: Color32::from_rgb(104, 104, 116),
    accent: Color32::from_rgb(110, 141, 249),
    accent_wash: Color32::from_rgb(31, 34, 56),
    on_accent: Color32::from_rgb(10, 10, 14),
};

/// Paper rather than pure white for the rail, so the content area is the
/// brightest thing on screen and the eye lands there first.
pub const LIGHT: Palette = Palette {
    dark: false,
    bg: Color32::from_rgb(255, 255, 255),
    sidebar: Color32::from_rgb(244, 244, 247),
    surface: Color32::from_rgb(247, 247, 250),
    surface_hover: Color32::from_rgb(238, 238, 243),
    surface_active: Color32::from_rgb(228, 228, 236),
    divider: Color32::from_rgb(226, 226, 233),
    text_primary: Color32::from_rgb(22, 22, 27),
    text_secondary: Color32::from_rgb(96, 96, 108),
    text_tertiary: Color32::from_rgb(140, 140, 152),
    accent: Color32::from_rgb(59, 92, 224),
    accent_wash: Color32::from_rgb(232, 236, 253),
    on_accent: Color32::from_rgb(255, 255, 255),
};

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_premultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

impl Palette {
    /// Blend two palettes. `t` of 0 is `a`, 1 is `b`.
    pub fn lerp(a: Palette, b: Palette, t: f32) -> Palette {
        Palette {
            // Past the midpoint the blend is closer to `b`, and egui's own
            // defaults should follow it rather than lag behind.
            dark: if t < 0.5 { a.dark } else { b.dark },
            bg: mix(a.bg, b.bg, t),
            sidebar: mix(a.sidebar, b.sidebar, t),
            surface: mix(a.surface, b.surface, t),
            surface_hover: mix(a.surface_hover, b.surface_hover, t),
            surface_active: mix(a.surface_active, b.surface_active, t),
            divider: mix(a.divider, b.divider, t),
            text_primary: mix(a.text_primary, b.text_primary, t),
            text_secondary: mix(a.text_secondary, b.text_secondary, t),
            text_tertiary: mix(a.text_tertiary, b.text_tertiary, t),
            accent: mix(a.accent, b.accent, t),
            accent_wash: mix(a.accent_wash, b.accent_wash, t),
            on_accent: mix(a.on_accent, b.on_accent, t),
        }
    }
}

thread_local! {
    /// The palette the accessors below report. Set once per frame, before any
    /// of the UI is built. Thread-local because eframe only ever builds the UI
    /// on one thread, and this avoids a lock on something read this often.
    static CURRENT: Cell<Palette> = const { Cell::new(DARK) };
}

/// Install the palette for this frame.
pub fn set(pal: Palette) {
    CURRENT.with(|c| c.set(pal));
}

/// Which end of the blend the user asked for, resolving `System` against what
/// Windows reports. Falls back to dark, which is what this app has always been.
pub fn resolve(want: UiTheme, ctx: &egui::Context) -> bool {
    match want {
        UiTheme::Dark => true,
        UiTheme::Light => false,
        UiTheme::System => ctx
            .system_theme()
            .map(|t| t == egui::Theme::Dark)
            .unwrap_or(true),
    }
}

/// Push the palette into egui's own style, so stock widgets — checkboxes,
/// sliders, combo boxes, scroll bars — are painted from it too.
///
/// Only called when the palette actually changes, which during a theme
/// cross-fade is every frame and the rest of the time is never.
pub fn apply_style(ctx: &egui::Context, pal: Palette) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.dark_mode = pal.dark;
    v.window_fill = pal.bg;
    v.panel_fill = pal.bg;
    v.extreme_bg_color = pal.surface;
    v.faint_bg_color = pal.surface;
    v.override_text_color = Some(pal.text_primary);
    v.hyperlink_color = pal.accent;
    v.selection.bg_fill = pal.accent;
    v.selection.stroke = Stroke::new(1.0, pal.on_accent);

    v.widgets.noninteractive.bg_fill = pal.surface;
    v.widgets.noninteractive.weak_bg_fill = pal.surface;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal.divider);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.text_secondary);
    v.widgets.inactive.bg_fill = pal.surface;
    v.widgets.inactive.weak_bg_fill = pal.surface;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.divider);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.text_primary);
    v.widgets.hovered.bg_fill = pal.surface_hover;
    v.widgets.hovered.weak_bg_fill = pal.surface_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.accent.linear_multiply(0.55));
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, pal.text_primary);
    v.widgets.active.bg_fill = pal.surface_active;
    v.widgets.active.weak_bg_fill = pal.surface_active;
    v.widgets.active.bg_stroke = Stroke::new(1.0, pal.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, pal.text_primary);
    v.widgets.open.bg_fill = pal.surface_hover;
    v.widgets.open.weak_bg_fill = pal.surface_hover;
    v.widgets.open.bg_stroke = Stroke::new(1.0, pal.divider);
    v.widgets.open.fg_stroke = Stroke::new(1.0, pal.text_primary);

    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.rounding = Rounding::same(7.0);
        w.expansion = 0.0;
    }

    ctx.set_style(style);
}

// -- accessors -----------------------------------------------------------
//
// One per token. They exist so a call site reads `theme::accent()` and not
// `theme::current().accent`, which at a hundred call sites is the difference
// between the colour layer being invisible and being in the way.

macro_rules! token {
    ($($name:ident),* $(,)?) => {
        $(
            #[inline]
            pub fn $name() -> Color32 {
                CURRENT.with(|c| c.get().$name)
            }
        )*
    };
}

token!(
    bg,
    sidebar,
    surface,
    surface_hover,
    divider,
    text_primary,
    text_secondary,
    text_tertiary,
    accent,
    accent_wash,
    on_accent,
);
