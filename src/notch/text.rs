//! DirectWrite plumbing for the notch.
//!
//! The interesting part is [`build_private_collection`]: the two typefaces
//! shipped in the repo are registered as a private, in-memory font collection
//! so the notch gets Plus Jakarta Sans without asking the user to install
//! anything. System font fallback still applies on top of the private
//! collection, so emoji, CJK and Devanagari in the marquee keep resolving.

use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteFactory5, IDWriteFontCollection,
    IDWriteInlineObject, IDWriteTextFormat, IDWriteTextLayout, IDWriteTextLayout1,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_TEXT_RANGE, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING, DWRITE_WORD_WRAPPING_NO_WRAP,
};

/// Typefaces bundled with the binary, exposed to DirectWrite as a private
/// collection. Keep the family names in sync with the actual `name` table.
const BUNDLED_FONTS: [(&[u8], &str); 2] = [
    (
        include_bytes!("../../PlusJakartaSans.ttf"),
        "Plus Jakarta Sans",
    ),
    (
        include_bytes!("../../NotoSansDevanagari.ttf"),
        "Noto Sans Devanagari",
    ),
];

/// Used whenever the configured family name is blank.
const FALLBACK_FAMILY: &str = "Segoe UI";

pub struct TextEngine {
    pub factory: IDWriteFactory,
    /// `None` when the private collection could not be built (pre-1607
    /// Windows, or a malformed font file). Everything degrades to system
    /// fonts rather than failing.
    private_collection: Option<IDWriteFontCollection>,
    private_families: Vec<String>,
}

unsafe impl Send for TextEngine {}
unsafe impl Sync for TextEngine {}

impl TextEngine {
    pub fn new() -> windows::core::Result<Self> {
        let factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };

        let (private_collection, private_families) = match build_private_collection(&factory) {
            Some(collection) => (
                Some(collection),
                BUNDLED_FONTS.iter().map(|(_, n)| n.to_string()).collect(),
            ),
            None => {
                eprintln!("[notch] private font collection unavailable, using system fonts");
                (None, Vec::new())
            }
        };

        Ok(Self {
            factory,
            private_collection,
            private_families,
        })
    }

    fn is_private(&self, family: &str) -> bool {
        self.private_families
            .iter()
            .any(|f| f.eq_ignore_ascii_case(family))
    }

    /// The family to actually ask DirectWrite for, given what the user picked.
    pub fn resolve_family(requested: &str) -> &str {
        if requested.trim().is_empty() {
            FALLBACK_FAMILY
        } else {
            requested
        }
    }

    pub fn format(
        &self,
        family: &str,
        size: f32,
        weight: DWRITE_FONT_WEIGHT,
    ) -> windows::core::Result<IDWriteTextFormat> {
        let family = Self::resolve_family(family);
        let family_hstr = HSTRING::from(family);
        let locale = HSTRING::from("en-us");

        // Only hand DirectWrite the private collection when the requested
        // family actually lives in it; otherwise the system collection is what
        // resolves "Segoe UI" and friends.
        let collection = if self.is_private(family) {
            self.private_collection.clone()
        } else {
            None
        };

        let format = unsafe {
            self.factory.CreateTextFormat(
                PCWSTR(family_hstr.as_ptr()),
                collection.as_ref(),
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size.max(1.0),
                PCWSTR(locale.as_ptr()),
            )?
        };

        unsafe {
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
        }

        Ok(format)
    }

    pub fn set_wrapping(format: &IDWriteTextFormat, wrapping: DWRITE_WORD_WRAPPING) {
        unsafe {
            let _ = format.SetWordWrapping(wrapping);
        }
    }

    /// Clip overflow with a real ellipsis glyph instead of a hard cut.
    pub fn set_ellipsis(&self, format: &IDWriteTextFormat) {
        unsafe {
            if let Ok(sign) = self.factory.CreateEllipsisTrimmingSign(format) {
                let sign: IDWriteInlineObject = sign;
                let trimming = DWRITE_TRIMMING {
                    granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter: 0,
                    delimiterCount: 0,
                };
                let _ = format.SetTrimming(&trimming, &sign);
            }
        }
    }

    pub fn layout(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        max_w: f32,
        max_h: f32,
    ) -> windows::core::Result<IDWriteTextLayout> {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            self.factory
                .CreateTextLayout(&utf16, format, max_w.max(1.0), max_h.max(1.0))
        }
    }

    /// Letter-spacing. Small tracked labels are unreadable without it, and it
    /// is the cheapest single thing that makes the panel look designed.
    pub fn set_tracking(layout: &IDWriteTextLayout, tracking: f32, char_count: u32) {
        if char_count == 0 {
            return;
        }
        if let Ok(layout1) = layout.cast::<IDWriteTextLayout1>() {
            let range = DWRITE_TEXT_RANGE {
                startPosition: 0,
                length: char_count,
            };
            unsafe {
                let _ = layout1.SetCharacterSpacing(0.0, tracking, 0.0, range);
            }
        }
    }

    /// `(width, height)` of the laid-out text, including trailing whitespace.
    pub fn measure(layout: &IDWriteTextLayout) -> (f32, f32) {
        unsafe {
            let mut metrics = std::mem::zeroed();
            if layout.GetMetrics(&mut metrics).is_err() {
                return (0.0, 0.0);
            }
            (
                metrics.widthIncludingTrailingWhitespace.max(metrics.width),
                metrics.height,
            )
        }
    }
}

/// Register the bundled TTFs with DirectWrite. Returns `None` on any platform
/// or API failure; every caller treats that as "use system fonts".
fn build_private_collection(factory: &IDWriteFactory) -> Option<IDWriteFontCollection> {
    let factory5: IDWriteFactory5 = factory.cast().ok()?;

    unsafe {
        let loader = factory5.CreateInMemoryFontFileLoader().ok()?;
        factory5.RegisterFontFileLoader(&loader).ok()?;

        let builder = factory5.CreateFontSetBuilder().ok()?;

        for (data, _) in BUNDLED_FONTS {
            let file = loader
                .CreateInMemoryFontFileReference(
                    &factory5,
                    data.as_ptr() as *const std::ffi::c_void,
                    data.len() as u32,
                    None,
                )
                .ok()?;
            builder.AddFontFile(&file).ok()?;
        }

        let font_set = builder.CreateFontSet().ok()?;
        let collection = factory5.CreateFontCollectionFromFontSet(&font_set).ok()?;
        Some(collection.into())
    }
}
