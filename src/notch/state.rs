//! Runtime state for the notch: what it is showing, how open it is, and
//! whether the user is currently typing into it.
//!
//! Nothing here touches Win32 or Direct2D — it is pure state so the open /
//! collapse / carousel behaviour can be reasoned about on its own.

use std::time::{Duration, Instant};

use crate::config::{AppConfig, SlideKind};
use crate::notch::anim::Spring;

/// Placeholder shown when the focus line has been cleared entirely.
const EMPTY_FOCUS: &str = "What are you working on?";

/// How long the click-through badge stays up after the chord.
const FLASH_SECONDS: f32 = 1.6;

pub struct NotchState {
    /// 0 = collapsed pill, 1 = fully open panel.
    pub expand: Spring,
    /// 0 = normal collapsed/expanded, 1 = dynamic alert pill toast.
    pub toast_progress: Spring,
    /// Continuous carousel position; settles on `active`.
    pub carousel: Spring,
    /// The slide the notch rests on.
    pub active: usize,

    pub hovered: bool,
    /// When the cursor left. Collapsing waits out a grace period so brushing
    /// the edge of the panel does not slam it shut.
    left_at: Option<Instant>,

    /// Set by the pin toggle. Holds the panel open regardless of hover, the
    /// same as `editing` does — pinning is just a user-requested version of
    /// "the cursor is still here".
    pub pinned: bool,

    pub editing: bool,
    pub edit_buffer: String,
    pub caret_on: bool,
    caret_clock: f32,

    /// Marquee scroll position, in pixels.
    pub marquee_offset: f32,

    /// Free-running clock for ambient motion (the accent pulse).
    pub elapsed: f32,

    /// Currently inspected notification in Notification Center expanded slide.
    pub selected_notification_id: Option<u64>,

    /// Set when something the user changed needs writing back to disk.
    pub dirty: bool,

    /// Seconds left on the click-through confirmation badge. The chord is
    /// invisible by nature — the clicks go somewhere else — so the notch says
    /// out loud which mode it just entered, then gets out of the way.
    pub click_through_flash: f32,
}

impl NotchState {
    pub fn new(cfg: &AppConfig) -> Self {
        let active = cfg.notch.clamped_active();
        Self {
            // Stiff enough to feel immediate, damped just under critical so it
            // arrives with the faintest settle rather than a dead stop.
            expand: Spring::new(0.0, 200.0, 26.0),
            toast_progress: Spring::new(0.0, 220.0, 28.0),
            carousel: Spring::new(active as f32, 230.0, 30.0),
            active,
            hovered: false,
            left_at: None,
            pinned: false,
            editing: false,
            edit_buffer: String::new(),
            caret_on: true,
            caret_clock: 0.0,
            marquee_offset: 0.0,
            elapsed: 0.0,
            selected_notification_id: None,
            dirty: false,
            click_through_flash: 0.0,
        }
    }

    /// Show the click-through badge for long enough to read and no longer.
    pub fn flash_click_through(&mut self) {
        self.click_through_flash = FLASH_SECONDS;
    }

    pub fn toggle_pinned(&mut self) {
        self.pinned = !self.pinned;
        // Unpinning while the cursor has already wandered off should start
        // the collapse grace period now, not carry on looking "hovered".
        if !self.pinned && !self.hovered {
            self.left_at = Some(Instant::now());
        }
    }

    /// Reset active slide to the user's default_collapsed setting upon collapse.
    pub fn apply_default_collapsed(
        &mut self,
        cfg: &AppConfig,
        media_active: bool,
        unread_notifs: bool,
    ) {
        use crate::config::CollapsedMode;
        let slides = cfg.notch.effective_slides();
        let target_kind = match cfg.notch.default_collapsed {
            CollapsedMode::LastActive => return,
            CollapsedMode::Status => SlideKind::Status,
            CollapsedMode::Clock => SlideKind::Clock,
            CollapsedMode::Marquee => SlideKind::Marquee,
            CollapsedMode::Wallpaper => SlideKind::Wallpaper,
            CollapsedMode::Media => SlideKind::Media,
            CollapsedMode::Notifications => SlideKind::Notifications,
            CollapsedMode::Usage => SlideKind::Usage,
            CollapsedMode::Auto => {
                if unread_notifs && slides.contains(&SlideKind::Notifications) {
                    SlideKind::Notifications
                } else if media_active && slides.contains(&SlideKind::Media) {
                    SlideKind::Media
                } else if slides.contains(&SlideKind::Clock) {
                    SlideKind::Clock
                } else {
                    SlideKind::Status
                }
            }
        };

        if let Some(idx) = slides.iter().position(|s| *s == target_kind) {
            if self.active != idx {
                self.active = idx;
            }
        }
    }

    /// Slow breathing applied to the accent dot, 0.75..1.0.
    pub fn pulse(&self) -> f32 {
        0.875 + 0.125 * (self.elapsed * 1.7).sin()
    }

    pub fn is_open(&self) -> bool {
        self.expand.value > 0.5
    }

    /// The focus string to render: the live edit buffer while typing, the
    /// saved value otherwise.
    pub fn display_focus(&self, cfg: &AppConfig) -> String {
        let raw = if self.editing {
            self.edit_buffer.as_str()
        } else {
            cfg.status.focus.as_str()
        };

        if raw.trim().is_empty() && !self.editing {
            EMPTY_FOCUS.to_string()
        } else {
            raw.to_string()
        }
    }

    /// Feed the hover result for this frame. Returns the expansion target.
    pub fn set_hovered(&mut self, hovered: bool, collapse_delay: Duration) -> f32 {
        self.hovered = hovered;

        if hovered {
            self.left_at = None;
        } else if self.left_at.is_none() {
            self.left_at = Some(Instant::now());
        }

        // Typing keeps the panel open even if the cursor wanders off, or the
        // field would vanish mid-sentence. Pinning does the same, on purpose.
        if hovered || self.editing || self.pinned {
            return 1.0;
        }

        match self.left_at {
            Some(at) if at.elapsed() >= collapse_delay => 0.0,
            Some(_) => 1.0,
            None => 0.0,
        }
    }

    pub fn tick(&mut self, cfg: &AppConfig, target_expand: f32, dt: f32) {
        self.elapsed += dt;
        self.click_through_flash = (self.click_through_flash - dt).max(0.0);

        self.expand.step(target_expand, dt);
        self.carousel.step(self.active as f32, dt);

        // The marquee only advances when it can be seen.
        let slides = cfg.notch.effective_slides();
        let marquee_visible = slides
            .get(self.active)
            .map(|s| *s == SlideKind::Marquee)
            .unwrap_or(false)
            || (self.expand.value > 0.05 && slides.contains(&SlideKind::Marquee));

        if marquee_visible {
            let speed = if cfg.animation.reverse {
                -cfg.animation.speed
            } else {
                cfg.animation.speed
            };
            self.marquee_offset += speed * dt;
            // Keep the accumulator small; the painter re-wraps it anyway.
            if self.marquee_offset.abs() > 1.0e6 {
                self.marquee_offset = 0.0;
            }
        }

        if self.editing {
            self.caret_clock += dt;
            if self.caret_clock >= 0.53 {
                self.caret_clock = 0.0;
                self.caret_on = !self.caret_on;
            }
        } else {
            self.caret_on = true;
            self.caret_clock = 0.0;
        }
    }

    /// Advance the carousel by `steps` slides, clamped to the ends.
    ///
    /// Deliberately clamped rather than wrapping: hitting a wall tells you
    /// where you are in the deck, whereas wrapping makes a four-slide carousel
    /// feel like an infinite scroll.
    pub fn step_slide(&mut self, steps: i32, cfg: &AppConfig) {
        let len = cfg.notch.effective_slides().len();
        if len == 0 {
            return;
        }
        let next = (self.active as i32 + steps).clamp(0, len as i32 - 1) as usize;
        if next != self.active {
            self.active = next;
            self.dirty = true;
            // Changing slides while typing would strand the edit.
            self.cancel_edit();
        }
    }

    pub fn clamp_active(&mut self, cfg: &AppConfig) {
        let len = cfg.notch.effective_slides().len();
        if len == 0 {
            return;
        }
        if self.active >= len {
            self.active = len - 1;
            self.carousel.set(self.active as f32);
        }
    }

    pub fn begin_edit(&mut self, cfg: &AppConfig) {
        if self.editing {
            return;
        }
        self.editing = true;
        self.edit_buffer = cfg.status.focus.clone();
        self.caret_on = true;
        self.caret_clock = 0.0;
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    /// Commit the buffer into `cfg`. Returns true when something changed.
    pub fn commit_edit(&mut self, cfg: &mut AppConfig) -> bool {
        if !self.editing {
            return false;
        }
        let next = self.edit_buffer.trim().to_string();
        self.editing = false;
        self.edit_buffer.clear();

        if next != cfg.status.focus {
            cfg.status.focus = next;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Handle one typed character. Control characters are filtered out here so
    /// the caller only deals with intent.
    pub fn type_char(&mut self, ch: char) {
        if !self.editing {
            return;
        }
        // Reasonable ceiling: the focus line is one thought, not a paragraph.
        if self.edit_buffer.chars().count() >= 160 {
            return;
        }
        if !ch.is_control() {
            self.edit_buffer.push(ch);
            self.caret_on = true;
            self.caret_clock = 0.0;
        }
    }

    pub fn backspace(&mut self) {
        if self.editing {
            self.edit_buffer.pop();
            self.caret_on = true;
            self.caret_clock = 0.0;
        }
    }
}
