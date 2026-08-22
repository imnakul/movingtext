//! The layered window a flash plays on.
//!
//! A full-monitor `WS_EX_LAYERED` popup that spends its life hidden: the
//! manager shows it for the length of a flash and hides it again, so between
//! flashes it costs nothing — not a repaint, not a hit-test, not a z-order
//! fight with anyone else's topmost window.

use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HDC,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, RegisterClassW,
    SetWindowLongW, SetWindowPos, ShowWindow, UpdateLayeredWindow, GWL_EXSTYLE, HMENU,
    HWND_NOTOPMOST, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::config::FlashConfig;
use crate::flash::renderer::FlashRenderer;
use crate::notch::backdrop;

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

const CLASS_NAME: &str = "VenuFlashClass";

pub struct FlashWindow {
    hwnd: HWND,
    renderer: FlashRenderer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

unsafe impl Send for FlashWindow {}
unsafe impl Sync for FlashWindow {}

impl FlashWindow {
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

    pub fn create(cfg: &FlashConfig) -> windows::core::Result<Self> {
        Self::register_class()?;

        let hwnd = unsafe {
            CreateWindowExW(
                Self::ex_style(cfg),
                PCWSTR(HSTRING::from(CLASS_NAME).as_ptr()),
                PCWSTR(HSTRING::from("VenuFlash").as_ptr()),
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

        let mut window = Self {
            hwnd,
            renderer: FlashRenderer::new()?,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        window.sync_geometry(cfg);

        Ok(window)
    }

    fn ex_style(cfg: &FlashConfig) -> windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE {
        let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        if cfg.always_on_top {
            ex_style |= WS_EX_TOPMOST;
        }
        if cfg.click_through {
            ex_style |= WS_EX_TRANSPARENT;
        }
        ex_style
    }

    /// Re-derive position, size and window styles from the config. Runs on
    /// every flash, so a monitor switch or a style toggle applies to the very
    /// next one rather than after a restart.
    pub fn sync_geometry(&mut self, cfg: &FlashConfig) {
        let monitors = crate::notch::window::monitor_rects();
        let rect = monitors
            .get(cfg.monitor_index.min(monitors.len().saturating_sub(1)))
            .copied()
            .unwrap_or(RECT {
                left: 0,
                top: 0,
                right: unsafe { GetSystemMetrics(SM_CXSCREEN) },
                bottom: unsafe { GetSystemMetrics(SM_CYSCREEN) },
            });

        self.x = rect.left;
        self.y = rect.top;
        self.width = (rect.right - rect.left).max(1) as u32;
        self.height = (rect.bottom - rect.top).max(1) as u32;

        unsafe {
            SetWindowLongW(self.hwnd, GWL_EXSTYLE, Self::ex_style(cfg).0 as i32);

            // The blurred backgrounds sample the screen behind the window, so
            // the window must be kept out of its own capture or each frame
            // would smear the last one into feedback. The trade-off — the
            // flash then also stays out of screenshots — is the same one the
            // notch's frosted theme makes.
            if cfg.bg_kind.samples_desktop() {
                backdrop::exclude_from_capture(self.hwnd);
            } else {
                backdrop::include_in_capture(self.hwnd);
            }

            let hwnd_insert_after = if cfg.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };

            // Deliberately not SWP_SHOWWINDOW: the window stays hidden until
            // the manager has something to put on it.
            let _ = SetWindowPos(
                self.hwnd,
                hwnd_insert_after,
                self.x,
                self.y,
                self.width as i32,
                self.height as i32,
                SWP_NOACTIVATE,
            );
        }
    }

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }

    /// Blank the surface *before* hiding, so the hidden window holds nothing:
    /// Windows would otherwise re-compose the last frame for a beat when the
    /// window is shown again for the next flash.
    pub fn hide(&mut self) {
        if let Err(e) = self.blank() {
            eprintln!("[flash] blank frame error: {e:?}");
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn render(
        &mut self,
        cfg: &FlashConfig,
        elapsed: f32,
        turn: u32,
    ) -> windows::core::Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        match self.renderer.render_frame(
            self.width,
            self.height,
            self.x,
            self.y,
            cfg,
            elapsed,
            turn,
        ) {
            Ok(Some(mem_dc)) => {
                self.push_frame(mem_dc, cfg.always_on_top);
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn blank(&mut self) -> windows::core::Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }
        match self.renderer.render_blank(self.width, self.height) {
            Ok(Some(mem_dc)) => {
                self.push_frame(mem_dc, true);
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn push_frame(&self, mem_dc: HDC, topmost: bool) {
        unsafe {
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

            let hwnd_insert_after = if topmost {
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

            if let Err(e) = UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                Some(&p_dst),
                Some(&p_size),
                mem_dc,
                Some(&p_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            ) {
                eprintln!("[flash] UpdateLayeredWindow error: {e:?}");
            }

            let _ = ReleaseDC(None, screen_dc);
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

impl Drop for FlashWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_invalid() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
