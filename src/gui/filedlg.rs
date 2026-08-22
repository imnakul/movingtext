//! A single, small wrapper around the common "Open File" dialog.
//!
//! `GetOpenFileNameW` is used rather than the newer `IFileOpenDialog` because
//! it needs no COM apartment on the calling thread — and the settings window
//! runs on eframe's thread, whose apartment model is not ours to assume.

use windows::core::PCWSTR;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Ask the user for an image. `title` names the dialog, so the flash picker
/// and the wallpaper picker can each say what they are for. Returns `None` if
/// they cancel.
pub fn pick_image(initial: &str, title: &str) -> Option<String> {
    // Null-separated, double-null-terminated pairs, as the API wants them.
    // Built here rather than as a const so the escaping stays readable.
    // Every image extension users actually meet, not only the ones WIC is
    // guaranteed to decode: the renderer degrades gracefully when a picked
    // file turns out not to load, but the dialog should never be the thing
    // that hides a picture from the user.
    let mut filter: Vec<u16> = Vec::new();
    for part in [
        "Images",
        "*.png;*.jpg;*.jpeg;*.jfif;*.bmp;*.gif;*.webp;*.tif;*.tiff;*.ico;*.avif;*.heic;*.heif;*.svg",
        "All files",
        "*.*",
    ] {
        filter.extend(part.encode_utf16());
        filter.push(0);
    }
    filter.push(0);

    // The dialog writes the chosen path back into this buffer, so it has to be
    // big enough for a long path and owned for the duration of the call.
    let mut buffer = vec![0u16; 1024];
    let trimmed = initial.trim();
    if !trimmed.is_empty() && trimmed.len() < 1000 {
        let src = wide(trimmed);
        buffer[..src.len()].copy_from_slice(&src);
    }

    let title = wide(title);

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: windows::core::PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        // NOCHANGEDIR matters: without it the dialog moves the whole process's
        // working directory, which would break every relative path we hold.
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if !ok.as_bool() {
        return None;
    }

    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..end]);
    if path.trim().is_empty() {
        None
    } else {
        Some(path)
    }
}
