use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeSelection {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaddingConfig {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Segoe UI".to_string(),
            size: 20.0,
            bold: true,
            italic: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub text_color: [f32; 4], // RGBA 0.0 - 1.0
    pub bg_color: [f32; 4],   // RGBA 0.0 - 1.0
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            text_color: [1.0, 0.9, 0.2, 1.0],   // Vibrant Gold / Yellow
            bg_color: [0.08, 0.08, 0.12, 0.85], // Sleek Dark Semi-Transparent
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimConfig {
    pub speed: f32, // pixels per second
    pub reverse: bool,
}

impl Default for AnimConfig {
    fn default() -> Self {
        Self {
            speed: 120.0,
            reverse: false,
        }
    }
}


// ---------------------------------------------------------------------------
// Notch
// ---------------------------------------------------------------------------

/// Horizontal anchor for the notch on its monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotchAlign {
    Left,
    Center,
    Right,
}

fn default_theme() -> NotchTheme {
    NotchTheme::Dark
}

fn default_true() -> bool {
    true
}

/// How the notch is finished.
///
/// This is a surface treatment, not a full re-skin: the accent stays yours and
/// the layout never changes. Only the panel and the type tones move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotchTheme {
    /// Near-black obsidian. The default, and the only one that truly vanishes
    /// into a laptop bezel.
    Dark,
    /// Bone-white panel with dark type, for light desktops.
    Light,
    /// Translucent, with the screen behind it blurred through the panel.
    Frosted,
}

impl NotchTheme {
    pub const ALL: [NotchTheme; 3] = [NotchTheme::Dark, NotchTheme::Light, NotchTheme::Frosted];

    pub fn label(self) -> &'static str {
        match self {
            NotchTheme::Dark => "Dark",
            NotchTheme::Light => "Light",
            NotchTheme::Frosted => "Frosted glass",
        }
    }

    /// The panel colour this theme wants. Applied when the user switches
    /// themes; they are free to tint it afterwards.
    pub fn default_surface(self) -> [f32; 4] {
        match self {
            NotchTheme::Dark => [0.031, 0.031, 0.043, 0.97],
            NotchTheme::Light => [0.965, 0.965, 0.976, 0.97],
            // Low alpha on purpose: the blurred capture behind it supplies
            // most of the body, and this is only the tint on top.
            NotchTheme::Frosted => [0.07, 0.07, 0.09, 0.55],
        }
    }
}

/// How the settings window is painted.
///
/// `System` follows whatever Windows reports for apps, so a machine set to
/// switch at sunset takes the window with it. The other two pin it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UiTheme {
    #[default]
    System,
    Light,
    Dark,
}

impl UiTheme {
    pub const ALL: [UiTheme; 3] = [UiTheme::System, UiTheme::Light, UiTheme::Dark];

    pub fn label(self) -> &'static str {
        match self {
            UiTheme::System => "System",
            UiTheme::Light => "Light",
            UiTheme::Dark => "Dark",
        }
    }
}

/// The faces the notch can show. The order of [`NotchConfig::slides`] is the
/// carousel order the scroll wheel walks through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideKind {
    /// Split view: what today is about (left) and today's short list (right).
    Status,
    /// Oversized clock plus the date.
    Clock,
    /// The classic scrolling marquee, now living inside the notch.
    Marquee,
    /// A picture you want to be reminded of, with an optional caption.
    Wallpaper,
    /// Whatever is currently playing in the system's media session, if
    /// anything: title, artist, art, and transport controls.
    Media,
}

impl SlideKind {
    pub const ALL: [SlideKind; 5] = [
        SlideKind::Status,
        SlideKind::Clock,
        SlideKind::Marquee,
        SlideKind::Wallpaper,
        SlideKind::Media,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SlideKind::Status => "Status",
            SlideKind::Clock => "Clock",
            SlideKind::Marquee => "Moving text",
            SlideKind::Wallpaper => "Wallpaper",
            SlideKind::Media => "Now Playing",
        }
    }
}

/// One line on today's short list. Deliberately not a to-do system: this is
/// only meant to hold the handful of things today is actually about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusItem {
    pub text: String,
    pub done: bool,
}

impl StatusItem {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            done: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusConfig {
    /// Small tracked label above the focus line, e.g. "TODAY".
    pub heading: String,
    /// The one thing being worked on right now. Editable inline from the notch.
    pub focus: String,
    /// Everything else today is about. Capped in the UI, not here.
    pub items: Vec<StatusItem>,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            heading: "TODAY".to_string(),
            focus: "Set what you are working on".to_string(),
            items: vec![
                StatusItem::new("Ship the notch overlay"),
                StatusItem::new("Review pull requests"),
                StatusItem::new("Write the release notes"),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperConfig {
    /// Absolute path to a PNG/JPG/BMP/GIF. Empty means "show the placeholder".
    pub path: String,
    pub caption: String,

    /// Which point of the image to keep in frame when it is cropped to the
    /// panel, as a 0..1 fraction of the image. 0.5/0.5 is dead centre.
    #[serde(default = "half")]
    pub focus_x: f32,
    #[serde(default = "half")]
    pub focus_y: f32,

    /// Extra magnification on top of the cover fit. 1.0 shows as much of the
    /// image as the panel shape allows.
    #[serde(default = "one")]
    pub zoom: f32,

    /// Panel size while the wallpaper slide is showing. Zero means "use the
    /// notch's normal expanded size"; a picture usually wants more room than a
    /// line of text does, so it gets to ask for its own.
    #[serde(default)]
    pub panel_width: u32,
    #[serde(default)]
    pub panel_height: u32,
}

fn half() -> f32 {
    0.5
}

fn one() -> f32 {
    1.0
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            caption: String::new(),
            focus_x: 0.5,
            focus_y: 0.5,
            zoom: 1.0,
            panel_width: 0,
            panel_height: 0,
        }
    }
}

impl WallpaperConfig {
    pub fn has_image(&self) -> bool {
        !self.path.trim().is_empty()
    }

    /// Clamp the framing controls into ranges the painter can rely on.
    pub fn sanitised(&self) -> (f32, f32, f32) {
        (
            self.focus_x.clamp(0.0, 1.0),
            self.focus_y.clamp(0.0, 1.0),
            self.zoom.clamp(1.0, 4.0),
        )
    }
}

/// Per-slide overrides for the moving-text slide.
///
/// A scrolling line wants a different shape than a clock does — more width,
/// less height, and its own type size. Every field is an override rather than
/// a value: `0` means "whatever the notch itself is set to", so the slide only
/// diverges where the user deliberately made it diverge, and the notch's own
/// settings keep working as the single place to change everything at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MarqueeConfig {
    /// Whether the line scrolls. Off, it simply sits there: the same message,
    /// held still. Motion is what makes a marquee readable at a glance from
    /// across the room, and also what makes it impossible to ignore while you
    /// are trying to work — so which one you want depends on the message.
    pub scroll: bool,
    pub collapsed_width: u32,
    pub collapsed_height: u32,
    pub panel_width: u32,
    pub panel_height: u32,
    /// Type size of the line in the open panel.
    pub font_size: f32,
    /// Type size of the line in the collapsed pill.
    pub pill_font_size: f32,
}

impl Default for MarqueeConfig {
    fn default() -> Self {
        Self {
            // Scrolling is the whole point of the slide; a static line is the
            // deliberate choice, not the starting state.
            scroll: true,
            collapsed_width: 0,
            collapsed_height: 0,
            panel_width: 0,
            panel_height: 0,
            font_size: 0.0,
            pill_font_size: 0.0,
        }
    }
}

impl MarqueeConfig {
    /// Type size for the open panel, or `fallback` when the user has not
    /// asked for one. Clamped because the value round-trips through JSON and
    /// a hand-edited zero-or-huge size would otherwise reach DirectWrite.
    pub fn panel_font(&self, fallback: f32) -> f32 {
        if self.font_size > 0.0 {
            self.font_size.clamp(8.0, 120.0)
        } else {
            fallback
        }
    }

    /// Type size for the collapsed pill, or `fallback`.
    pub fn pill_font(&self, fallback: f32) -> f32 {
        if self.pill_font_size > 0.0 {
            self.pill_font_size.clamp(6.0, 64.0)
        } else {
            fallback
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotchConfig {
    pub enabled: bool,
    pub monitor_index: usize,
    pub align: NotchAlign,
    /// Nudge from the anchor, in pixels. Positive is right.
    pub offset_x: i32,
    /// Distance from the top of the monitor. 0 keeps the notch fused to the
    /// bezel and enables the concave shoulders; anything else detaches it.
    pub offset_y: i32,
    pub collapsed_width: u32,
    pub collapsed_height: u32,
    pub expanded_width: u32,
    pub expanded_height: u32,
    pub slides: Vec<SlideKind>,
    /// The slide the notch rests on when collapsed. Persisted so the notch
    /// comes back showing whatever you last left it on.
    pub active_slide: usize,
    pub accent: [f32; 4],
    pub surface: [f32; 4],
    #[serde(default = "default_theme")]
    pub theme: NotchTheme,
    pub font_family: String,
    pub clock_24h: bool,
    /// Grace period after the cursor leaves before collapsing, so brushing
    /// past the edge of the panel does not slam it shut.
    pub collapse_delay_ms: u32,
    pub scroll_to_switch: bool,
    pub always_on_top: bool,
    /// When on, the notch stops taking mouse clicks: they land on whatever is
    /// underneath instead. It still opens on hover and still answers the
    /// wheel, so the deck stays usable — only clicking through to the window
    /// below changes.
    ///
    /// Toggled by pressing the left and right mouse buttons together with the
    /// cursor over the notch, because once click-through is on there is no
    /// button left to press to turn it back off.
    #[serde(default)]
    pub click_through: bool,
}

impl Default for NotchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor_index: 0,
            align: NotchAlign::Center,
            offset_x: 0,
            offset_y: 0,
            collapsed_width: 188,
            collapsed_height: 34,
            expanded_width: 760,
            expanded_height: 208,
            slides: SlideKind::ALL.to_vec(),
            active_slide: 0,
            accent: [1.0, 0.604, 0.235, 1.0],  // ember
            surface: [0.031, 0.031, 0.043, 0.97], // obsidian
            theme: NotchTheme::Dark,
            font_family: "Plus Jakarta Sans".to_string(),
            clock_24h: false,
            collapse_delay_ms: 220,
            scroll_to_switch: true,
            always_on_top: true,
            click_through: false,
        }
    }
}

impl NotchConfig {
    /// Slides, guaranteed non-empty, so the carousel always has something to
    /// land on even if every slide was unchecked in settings.
    pub fn effective_slides(&self) -> Vec<SlideKind> {
        if self.slides.is_empty() {
            vec![SlideKind::Clock]
        } else {
            self.slides.clone()
        }
    }

    pub fn clamped_active(&self) -> usize {
        let len = self.effective_slides().len();
        self.active_slide.min(len.saturating_sub(1))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub text: String,
    /// Master switch for the edge marquee — the strips of scrolling text
    /// around the screen border, which are a separate overlay from the notch.
    /// Off, every strip is torn down regardless of which edges are ticked, so
    /// the edge selection is preserved for when it comes back.
    #[serde(default = "default_true")]
    pub overlay_enabled: bool,
    #[serde(default = "default_phrase_spacing")]
    pub phrase_spacing: u32,
    pub edges: EdgeSelection,
    pub padding: PaddingConfig,
    pub thickness: u32,
    pub font: FontConfig,
    pub colors: ColorConfig,
    pub animation: AnimConfig,
    pub click_through: bool,
    pub always_on_top: bool,
    pub monitor_index: usize,
    #[serde(default)]
    pub notch: NotchConfig,
    #[serde(default)]
    pub status: StatusConfig,
    #[serde(default)]
    pub wallpaper: WallpaperConfig,
    #[serde(default)]
    pub marquee: MarqueeConfig,
    /// How the settings window itself is painted. Nothing to do with the
    /// overlays — this is only the chrome around the controls.
    #[serde(default)]
    pub ui_theme: UiTheme,
}

fn default_phrase_spacing() -> u32 {
    6
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            text: "⭐ REMINDER: Stay focused & stay hydrated! • MovingText Desktop Marquee ⭐"
                .to_string(),
            overlay_enabled: true,
            phrase_spacing: 6,
            edges: EdgeSelection::default(),
            padding: PaddingConfig::default(),
            thickness: 36,
            font: FontConfig::default(),
            colors: ColorConfig::default(),
            animation: AnimConfig::default(),
            click_through: true,
            always_on_top: true,
            monitor_index: 0,
            notch: NotchConfig::default(),
            status: StatusConfig::default(),
            wallpaper: WallpaperConfig::default(),
            marquee: MarqueeConfig::default(),
            ui_theme: UiTheme::default(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(mut path) = dirs::config_dir() {
            path.push("movingtext");
            let _ = fs::create_dir_all(&path);
            path.push("config.json");
            path
        } else {
            PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }
        let default_cfg = Self::default();
        default_cfg.save();
        default_cfg
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}
