//! The live blur behind the frosted theme.
//!
//! Real frosted glass needs whatever is behind the notch, blurred. There is no
//! way to sample the desktop from inside a layered window, so the desktop is
//! captured with GDI instead — and the blur is bought for free by capturing at
//! a fraction of the size and letting Direct2D scale it back up bilinearly.
//! A box-average down and a smooth interpolation up is, at this radius, hard
//! to tell from a real Gaussian.
//!
//! Two things make this safe:
//!
//! * The notch window sets `WDA_EXCLUDEFROMCAPTURE`, so the capture does not
//!   contain the notch itself. Without that, each frame would sample the
//!   previous one and smear into feedback within a second.
//! * Everything here degrades to `None`. If the capture or the upload fails,
//!   the painter simply skips the backdrop and the frosted theme falls back to
//!   its plain translucent surface, which still looks deliberate.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap, ID2D1DCRenderTarget, D2D1_BITMAP_PROPERTIES,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HALFTONE,
    HBITMAP, HDC, HGDIOBJ, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
};

/// How far the capture is shrunk before being scaled back up. Larger is
/// blurrier and cheaper; past about 10 the blur starts losing the shape of
/// what is behind it, which reads as fog rather than glass.
const DOWNSCALE: i32 = 8;

/// Keep the captured strip out of these ranges and the arithmetic stays sane
/// even if a caller asks for something absurd.
const MIN_DIM: i32 = 2;
const MAX_DIM: i32 = 512;

pub struct Backdrop {
    dc: HDC,
    dib: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    w: i32,
    h: i32,
}

impl Backdrop {
    /// Buffer pixels needed for a region of this size. Kept in one place so
    /// the capacity check and the per-frame footprint can never disagree.
    fn extent(width: i32, height: i32) -> (i32, i32) {
        (
            (width / DOWNSCALE).clamp(MIN_DIM, MAX_DIM),
            (height / DOWNSCALE).clamp(MIN_DIM, MAX_DIM),
        )
    }

    /// Allocate a capture surface with room for a region of `width` x `height`
    /// screen pixels. The surface is a *capacity*: smaller regions use its
    /// top-left corner rather than forcing a reallocation, which matters
    /// because the notch changes size on every frame of the open animation.
    /// Returns `None` if GDI refuses, which is not worth retrying.
    fn new(width: i32, height: i32) -> Option<Self> {
        let (w, h) = Self::extent(width, height);

        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return None;
            }

            // Negative height: top-down rows, matching what Direct2D expects.
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;

            if bits.is_null() {
                let _ = DeleteObject(dib);
                let _ = DeleteDC(dc);
                return None;
            }

            let previous = SelectObject(dc, dib);
            SetStretchBltMode(dc, HALFTONE);

            Some(Self {
                dc,
                dib,
                previous,
                bits: bits as *mut u8,
                w,
                h,
            })
        }
    }

    /// Pull the desktop under `(x, y, width, height)` into the top-left
    /// `uw` x `uh` corner of the capture buffer.
    fn capture(&self, x: i32, y: i32, width: i32, height: i32, uw: i32, uh: i32) -> bool {
        unsafe {
            let screen = GetDC(None);
            if screen.is_invalid() {
                return false;
            }

            let ok = StretchBlt(self.dc, 0, 0, uw, uh, screen, x, y, width, height, SRCCOPY)
                .as_bool();

            ReleaseDC(None, screen);

            if ok {
                // The desktop has no alpha channel; the DIB's fourth byte comes
                // back as zero, which a premultiplied target would read as
                // fully transparent. Force it opaque — only over the rows and
                // columns actually written, since the rest is stale.
                for row in 0..uh {
                    let base = (row * self.w) as usize * 4;
                    for col in 0..uw as usize {
                        *self.bits.add(base + col * 4 + 3) = 0xFF;
                    }
                }
            }

            ok
        }
    }

    /// Upload the used corner of the capture as a Direct2D bitmap. The stride
    /// is the buffer's full width, so a partly-filled buffer uploads as the
    /// sub-rectangle that was actually captured.
    fn upload(&self, t: &ID2D1DCRenderTarget, uw: i32, uh: i32) -> Option<ID2D1Bitmap> {
        let props = D2D1_BITMAP_PROPERTIES {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
        };

        let size = windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
            width: uw as u32,
            height: uh as u32,
        };

        unsafe {
            t.CreateBitmap(
                size,
                Some(self.bits as *const std::ffi::c_void),
                (self.w * 4) as u32,
                &props,
            )
            .ok()
        }
    }
}

impl Drop for Backdrop {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.dib);
            let _ = DeleteDC(self.dc);
        }
    }
}

/// Owns the capture surface across frames and hands the painter a fresh
/// bitmap. Held by the painter so the GDI objects are allocated once.
#[derive(Default)]
pub struct BackdropCache {
    inner: Option<Backdrop>,
    /// Buffer pixels the current surface can hold. Grow-only: the notch
    /// resizes on every frame of the open animation, and reallocating a DIB
    /// sixty times a second is the one thing here that would actually cost.
    capacity: (i32, i32),
    /// Set once the surface has failed, so a machine that cannot do this is
    /// not asked sixty times a second.
    broken: bool,
}

impl BackdropCache {
    /// Capture the desktop under the given screen rect and return it as a
    /// bitmap ready to be scaled up over the notch's silhouette.
    ///
    /// The bitmap is much smaller than the region, so the two floats returned
    /// alongside it are the x and y factors the caller must scale it by.
    pub fn sample(
        &mut self,
        t: &ID2D1DCRenderTarget,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Option<(ID2D1Bitmap, f32, f32)> {
        if self.broken || width <= 0 || height <= 0 {
            return None;
        }

        let (uw, uh) = Backdrop::extent(width, height);

        if self.inner.is_none() || uw > self.capacity.0 || uh > self.capacity.1 {
            // Grow to the union so a notch that gets wider and then taller
            // does not thrash between two shapes.
            let want_w = uw.max(self.capacity.0);
            let want_h = uh.max(self.capacity.1);
            self.inner = Backdrop::new(want_w * DOWNSCALE, want_h * DOWNSCALE);

            match self.inner.as_ref() {
                Some(b) => self.capacity = (b.w, b.h),
                None => {
                    eprintln!("[notch] backdrop capture unavailable; frosted theme will not blur");
                    self.broken = true;
                    self.capacity = (0, 0);
                    return None;
                }
            }
        }

        let backdrop = self.inner.as_ref()?;
        if !backdrop.capture(x, y, width, height, uw, uh) {
            return None;
        }

        let bitmap = backdrop.upload(t, uw, uh)?;
        // The captured corner always covers the whole region, so these are the
        // factors that take the small bitmap back up to full size.
        Some((bitmap, width as f32 / uw as f32, height as f32 / uh as f32))
    }
}

/// Ask the compositor to leave this window out of screen captures.
///
/// Without it the backdrop would sample the notch's own last frame and smear.
/// Returns false on Windows older than 10 2004, where the flag does not exist
/// — the caller then leaves the blur off rather than showing feedback.
///
/// The trade-off is worth stating plainly: while this is on, the notch will
/// not appear in the user's own screenshots or screen shares either.
pub fn exclude_from_capture(hwnd: HWND) -> bool {
    unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE).is_ok() }
}

/// Put the window back into screen captures, for when the user leaves the
/// frosted theme.
pub fn include_in_capture(hwnd: HWND) -> bool {
    unsafe { SetWindowDisplayAffinity(hwnd, WDA_NONE).is_ok() }
}
