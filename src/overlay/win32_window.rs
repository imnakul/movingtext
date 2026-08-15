use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, RegisterClassW,
    SetWindowLongW, SetWindowPos, ShowWindow, UpdateLayeredWindow, GWL_EXSTYLE, HMENU,
    HWND_NOTOPMOST, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::config::AppConfig;
use crate::overlay::d2d_renderer::{D2DRenderer, Edge};

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

const CLASS_NAME: &str = "VenuOverlayClass";

pub struct Win32OverlayWindow {
    hwnd: HWND,
    renderer: D2DRenderer,
    pub edge: Edge,
    offset: f32,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

unsafe impl Send for Win32OverlayWindow {}
unsafe impl Sync for Win32OverlayWindow {}

impl Win32OverlayWindow {
    fn register_class() -> windows::core::Result<()> {
        if CLASS_REGISTERED.load(Ordering::SeqCst) {
            return Ok(());
        }

        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class_name_hstr = HSTRING::from(CLASS_NAME);
            let wnd_class = WNDCLASSW {
                hInstance: instance.into(),
                lpszClassName: PCWSTR(class_name_hstr.as_ptr()),
                lpfnWndProc: Some(Self::wnd_proc),
                ..Default::default()
            };

            RegisterClassW(&wnd_class);
            CLASS_REGISTERED.store(true, Ordering::SeqCst);
        }

        Ok(())
    }

    pub fn create(edge: Edge, config: &AppConfig) -> windows::core::Result<Self> {
        Self::register_class()?;

        let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        if config.always_on_top {
            ex_style |= WS_EX_TOPMOST;
        }
        if config.click_through {
            ex_style |= WS_EX_TRANSPARENT;
        }

        let class_name_hstr = HSTRING::from(CLASS_NAME);
        let title_hstr = HSTRING::from("VenuOverlay");

        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                PCWSTR(class_name_hstr.as_ptr()),
                PCWSTR(title_hstr.as_ptr()),
                WS_POPUP,
                0,
                0,
                100,
                100,
                None,
                HMENU::default(),
                GetModuleHandleW(None)?,
                None,
            )?
        };

        let renderer = D2DRenderer::new()?;

        let mut window = Self {
            hwnd,
            renderer,
            edge,
            offset: 0.0,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
        };

        window.recalculate_geometry(config);
        window.show(true);

        Ok(window)
    }

    pub fn recalculate_geometry(&mut self, config: &AppConfig) {
        unsafe {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);

            let pad = &config.padding;
            let thick = config.thickness as i32;

            let (x, y, w, h) = match self.edge {
                Edge::Top => (
                    pad.left as i32,
                    pad.top as i32,
                    (screen_w - pad.left as i32 - pad.right as i32).max(1),
                    thick,
                ),
                Edge::Bottom => (
                    pad.left as i32,
                    (screen_h - pad.bottom as i32 - thick).max(0),
                    (screen_w - pad.left as i32 - pad.right as i32).max(1),
                    thick,
                ),
                Edge::Left => (
                    pad.left as i32,
                    pad.top as i32,
                    thick,
                    (screen_h - pad.top as i32 - pad.bottom as i32).max(1),
                ),
                Edge::Right => (
                    (screen_w - pad.right as i32 - thick).max(0),
                    pad.top as i32,
                    thick,
                    (screen_h - pad.top as i32 - pad.bottom as i32).max(1),
                ),
            };

            self.x = x;
            self.y = y;
            self.width = w as u32;
            self.height = h as u32;

            // Update window styles (click-through & always-on-top)
            let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            if config.always_on_top {
                ex_style |= WS_EX_TOPMOST;
            }
            if config.click_through {
                ex_style |= WS_EX_TRANSPARENT;
            }

            SetWindowLongW(self.hwnd, GWL_EXSTYLE, ex_style.0 as i32);

            let hwnd_insert_after = if config.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };

            let _ = SetWindowPos(
                self.hwnd,
                hwnd_insert_after,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn update_and_render(&mut self, config: &AppConfig, dt: f32) -> windows::core::Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let speed = if config.animation.reverse {
            -config.animation.speed
        } else {
            config.animation.speed
        };

        self.offset += speed * dt;
        if self.offset < 0.0 {
            self.offset += 100000.0;
        }

        match self
            .renderer
            .render_frame(self.width, self.height, config, self.offset, self.edge)
        {
            Ok(Some(mem_dc)) => unsafe {
                let screen_dc = GetDC(None);
                let p_dst = POINT {
                    x: self.x,
                    y: self.y,
                };
                let p_size = SIZE {
                    cx: self.width as i32,
                    cy: self.height as i32,
                };
                let p_src = POINT { x: 0, y: 0 };

                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };

                let hwnd_insert_after = if config.always_on_top {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                };

                let _ = SetWindowPos(
                    self.hwnd,
                    hwnd_insert_after,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );

                let res = UpdateLayeredWindow(
                    self.hwnd,
                    screen_dc,
                    Some(&p_dst),
                    Some(&p_size),
                    mem_dc,
                    Some(&p_src),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );
                if let Err(e) = res {
                    eprintln!("[Win32OverlayWindow] UpdateLayeredWindow error: {:?}", e);
                }

                let _ = ReleaseDC(None, screen_dc);
            },
            Ok(None) => {}
            Err(e) => {
                eprintln!("[Win32OverlayWindow] render_frame error: {:?}", e);
            }
        }

        Ok(())
    }

    pub fn show(&self, visible: bool) {
        unsafe {
            if visible {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            } else {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl Drop for Win32OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_invalid() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
