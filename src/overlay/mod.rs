pub mod d2d_renderer;
pub mod win32_window;

use std::collections::HashMap;

use crate::config::AppConfig;
use d2d_renderer::Edge;
use win32_window::Win32OverlayWindow;

pub struct OverlayManager {
    windows: HashMap<Edge, Win32OverlayWindow>,
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    pub fn sync_windows(&mut self, config: &AppConfig) {
        // The master switch is folded in here rather than short-circuiting the
        // caller, so turning it off takes the existing strips down through the
        // same path that unticking every edge would.
        let on = config.overlay_enabled;
        let edges_state = [
            (Edge::Top, on && config.edges.top),
            (Edge::Bottom, on && config.edges.bottom),
            (Edge::Left, on && config.edges.left),
            (Edge::Right, on && config.edges.right),
        ];

        for (edge, enabled) in edges_state {
            if enabled {
                if let Some(win) = self.windows.get_mut(&edge) {
                    win.recalculate_geometry(config);
                } else {
                    match Win32OverlayWindow::create(edge, config) {
                        Ok(win) => {
                            eprintln!("[OverlayManager] Created window for {:?}", edge);
                            self.windows.insert(edge, win);
                        }
                        Err(e) => {
                            eprintln!(
                                "[OverlayManager] Failed to create window for {:?}: {:?}",
                                edge, e
                            );
                        }
                    }
                }
            } else if self.windows.remove(&edge).is_some() {
                eprintln!("[OverlayManager] Removed window for {:?}", edge);
            }
        }
    }

    pub fn render_tick(&mut self, config: &AppConfig, dt: f32) {
        self.sync_windows(config);

        for (edge, win) in self.windows.iter_mut() {
            if let Err(e) = win.update_and_render(config, dt) {
                eprintln!("[OverlayManager] Render error for {:?}: {:?}", edge, e);
            }
        }
    }
}
