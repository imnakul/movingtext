//! A Direct2D render target backed by a top-down 32-bit DIB.
//!
//! Layered windows are updated with `UpdateLayeredWindow`, which wants an HDC
//! holding premultiplied BGRA. So we render through an `ID2D1DCRenderTarget`
//! bound to a memory DC that owns a DIB section, then hand that DC to the
//! window. The overlay marquee has its own copy of this dance; the notch keeps
//! a separate one so the two can evolve independently.

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
};

pub struct D2DSurface {
    pub factory: ID2D1Factory,
    target: Option<ID2D1DCRenderTarget>,
    mem_dc: HDC,
    hbitmap: HBITMAP,
    old_hbitmap: HBITMAP,
    width: u32,
    height: u32,
    /// Bumped every time the render target is rebuilt. Device-dependent
    /// resources cached elsewhere (bitmaps, brushes) compare against this to
    /// know when they have gone stale.
    generation: u64,
}

unsafe impl Send for D2DSurface {}
unsafe impl Sync for D2DSurface {}

impl D2DSurface {
    pub fn new() -> windows::core::Result<Self> {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };

        Ok(Self {
            factory,
            target: None,
            mem_dc: HDC::default(),
            hbitmap: HBITMAP::default(),
            old_hbitmap: HBITMAP::default(),
            width: 0,
            height: 0,
            generation: 0,
        })
    }

    pub fn target(&self) -> Option<&ID2D1DCRenderTarget> {
        self.target.as_ref()
    }

    pub fn dc(&self) -> HDC {
        self.mem_dc
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Guarantee a target of exactly `width` x `height`. Cheap no-op when the
    /// size is unchanged, which is the common case: the notch window is fixed
    /// at its expanded footprint and only the painted shape animates.
    pub fn ensure(&mut self, width: u32, height: u32) -> windows::core::Result<()> {
        if self.width == width
            && self.height == height
            && !self.mem_dc.is_invalid()
            && self.target.is_some()
        {
            return Ok(());
        }

        self.release();

        if width == 0 || height == 0 {
            return Ok(());
        }

        unsafe {
            let mem_dc = CreateCompatibleDC(None);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbitmap = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;
            let old_hbitmap = SelectObject(mem_dc, hbitmap);

            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            let target: ID2D1DCRenderTarget = self.factory.CreateDCRenderTarget(&props)?;
            let rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            target.BindDC(mem_dc, &rect)?;

            // ClearType cannot work on a surface with real transparency; it
            // would fringe every glyph against whatever is behind the window.
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);

            self.mem_dc = mem_dc;
            self.hbitmap = hbitmap;
            self.old_hbitmap = HBITMAP(old_hbitmap.0);
            self.width = width;
            self.height = height;
            self.target = Some(target);
            self.generation = self.generation.wrapping_add(1);
        }

        Ok(())
    }

    fn release(&mut self) {
        unsafe {
            self.target = None;
            if !self.mem_dc.is_invalid() {
                if !self.old_hbitmap.is_invalid() {
                    SelectObject(self.mem_dc, self.old_hbitmap);
                }
                if !self.hbitmap.is_invalid() {
                    let _ = DeleteObject(self.hbitmap);
                }
                let _ = DeleteDC(self.mem_dc);
            }
            self.mem_dc = HDC::default();
            self.hbitmap = HBITMAP::default();
            self.old_hbitmap = HBITMAP::default();
            self.width = 0;
            self.height = 0;
        }
    }
}

impl Drop for D2DSurface {
    fn drop(&mut self) {
        self.release();
    }
}
