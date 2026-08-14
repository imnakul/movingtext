//! The mouse hook, on a thread of its own.
//!
//! A `WH_MOUSE_LL` hook is called *synchronously on the thread that installed
//! it*, for every mouse event in the system. Windows delivers it through that
//! thread's message queue and waits for the callback to return before the
//! event continues to whatever is underneath. So if the installing thread is
//! busy — rendering, or sleeping between frames — the cursor stalls for as
//! long as the thread takes to get back to its pump. That is a system-wide
//! stutter, not a stutter in our window.
//!
//! The hook therefore lives here, on a dedicated thread whose entire job is to
//! block in `GetMessageW`. The callback touches nothing but atomics and
//! returns immediately, so it never becomes the slow link in mouse input.
//! Everything the callback needs to make a decision is published as plain
//! integers by the render thread.
//!
//! It watches for two things: the wheel, for switching slides, and a
//! left-plus-right button chord over the notch, which toggles click-through.
//! The chord has to be caught here rather than in the window procedure,
//! because the whole point of click-through is that the window stops
//! receiving mouse messages — once it is on, no click could ever reach the
//! notch to turn it back off again.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Once;

use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WINDOWS_HOOK_ID, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

/// One wheel notch, in the units Windows reports.
const WHEEL_DELTA: i32 = 120;

/// Accumulated notches waiting to be consumed by the render thread.
static WHEEL: AtomicI32 = AtomicI32::new(0);

/// Screen-space bounds of the notch. Only read by the callback.
static BOUND_L: AtomicI32 = AtomicI32::new(0);
static BOUND_T: AtomicI32 = AtomicI32::new(0);
static BOUND_R: AtomicI32 = AtomicI32::new(0);
static BOUND_B: AtomicI32 = AtomicI32::new(0);

/// False whenever the panel is closed or wheel-switching is turned off, in
/// which case the callback does nothing at all beyond a single atomic load.
static WHEEL_ACTIVE: AtomicBool = AtomicBool::new(false);

/// False whenever there is no notch on screen to toggle.
static CHORD_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Live button state, tracked here because a click-through window never sees
/// the presses itself.
static L_DOWN: AtomicBool = AtomicBool::new(false);
static R_DOWN: AtomicBool = AtomicBool::new(false);

/// Latched when both buttons were held together over the notch, until the
/// render thread consumes it.
static CHORD: AtomicBool = AtomicBool::new(false);

/// While a chord is in progress every remaining button event belongs to it and
/// must not reach anything else — otherwise letting go would drop a stray
/// right-click on whatever is underneath.
static SWALLOWING: AtomicBool = AtomicBool::new(false);

static INSTALL: Once = Once::new();

/// Install the hook on its own thread. Idempotent; safe to call every frame.
pub fn install() {
    INSTALL.call_once(|| {
        std::thread::Builder::new()
            .name("notch-mouse-hook".into())
            .spawn(|| unsafe {
                let instance = match GetModuleHandleW(None) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("[notch] hook thread: no module handle: {e:?}");
                        return;
                    }
                };

                let hook = match SetWindowsHookExW(
                    WINDOWS_HOOK_ID(WH_MOUSE_LL.0),
                    Some(proc),
                    instance,
                    0,
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!(
                            "[notch] mouse hook unavailable ({e:?}); \
                             wheel switching and the click-through chord are off"
                        );
                        return;
                    }
                };
                let _ = hook;

                // A low-level hook is only serviced while its thread pumps.
                // This loop never exits; the thread dies with the process.
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
            })
            .ok();
    });
}

/// Publish what the callback needs to know, once per frame.
///
/// `wheel` gates slide switching and `chord` gates the click-through toggle.
/// They are separate because the wheel only matters while the panel is open,
/// whereas the chord has to keep working when it is shut.
pub fn publish(wheel: bool, chord: bool, left: i32, top: i32, right: i32, bottom: i32) {
    BOUND_L.store(left, Ordering::Relaxed);
    BOUND_T.store(top, Ordering::Relaxed);
    BOUND_R.store(right, Ordering::Relaxed);
    BOUND_B.store(bottom, Ordering::Relaxed);
    WHEEL_ACTIVE.store(wheel, Ordering::Release);
    CHORD_ACTIVE.store(chord, Ordering::Release);
}

/// Stand everything down, for when there is no notch at all.
pub fn silence() {
    publish(false, false, 0, 0, 0, 0);
}

/// Consume everything scrolled since the last call.
pub fn take_wheel() -> i32 {
    WHEEL.swap(0, Ordering::AcqRel)
}

/// Consume a pending left-plus-right press over the notch, if there was one.
pub fn take_chord() -> bool {
    CHORD.swap(false, Ordering::AcqRel)
}

/// Whether the published bounds contain this point.
fn within(pt: POINT) -> bool {
    pt.x >= BOUND_L.load(Ordering::Relaxed)
        && pt.x <= BOUND_R.load(Ordering::Relaxed)
        && pt.y >= BOUND_T.load(Ordering::Relaxed)
        && pt.y <= BOUND_B.load(Ordering::Relaxed)
}

unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    // Moves are the overwhelming majority of what arrives here and match
    // neither arm, so the common case costs one integer compare.
    match wparam.0 as u32 {
        WM_MOUSEWHEEL if WHEEL_ACTIVE.load(Ordering::Acquire) => {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            if within(info.pt) {
                let delta = ((info.mouseData >> 16) & 0xFFFF) as i16 as i32;
                WHEEL.fetch_add(delta / WHEEL_DELTA, Ordering::AcqRel);
                // Swallow, so the window underneath does not scroll as well.
                return LRESULT(1);
            }
        }

        msg @ (WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP) => {
            let left = msg == WM_LBUTTONDOWN || msg == WM_LBUTTONUP;
            let down = msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN;

            // Record first, so the "is the other one held?" test below reads a
            // state that already includes this event.
            if left {
                L_DOWN.store(down, Ordering::Release);
            } else {
                R_DOWN.store(down, Ordering::Release);
            }

            if SWALLOWING.load(Ordering::Acquire) {
                // Both buttons back up: the chord is over.
                if !L_DOWN.load(Ordering::Acquire) && !R_DOWN.load(Ordering::Acquire) {
                    SWALLOWING.store(false, Ordering::Release);
                }
                return LRESULT(1);
            }

            if down && CHORD_ACTIVE.load(Ordering::Acquire) {
                let other = if left {
                    R_DOWN.load(Ordering::Acquire)
                } else {
                    L_DOWN.load(Ordering::Acquire)
                };
                let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                if other && within(info.pt) {
                    CHORD.store(true, Ordering::Release);
                    SWALLOWING.store(true, Ordering::Release);
                    return LRESULT(1);
                }
            }
        }

        _ => {}
    }

    CallNextHookEx(None, code, wparam, lparam)
}
