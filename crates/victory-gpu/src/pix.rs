//! Diagnostics: one frame of the title captured with PIX on Windows.
//!
//! `VICTORY_PIX=<full path of WinPixGpuCapturer.dll>` loads PIX's GPU
//! capturer into this process before the host device is made (its
//! documented programmatic use: it hooks the D3D12 interfaces created after
//! it is loaded). `VICTORY_PIX_FRAME=<n>` (2000 by default: the cell window
//! of a standard run) then asks it for the frames after present `n`,
//! `VICTORY_PIX_FRAMES` of them (1 by default), into `VICTORY_PIX_OUT` (by
//! default `victory_pix\frame.wpix` under the temporary folder). The capture
//! opens in PIX (`WinPix.exe`) or `pixtool open-capture`.
//!
//! Nothing is loaded unless the variable gives the full path of a file: a
//! bare name is refused, so neither the current folder nor the title's is
//! ever searched for it.

use std::sync::OnceLock;

/// `HRESULT CaptureNextFrame(PCWSTR fileName, UINT32 numFrames)`, the
/// capturer's export behind `PIXGpuCaptureNextFrames` of `pix3_win.h`.
type CaptureNextFrame = unsafe extern "system" fn(*const u16, u32) -> i32;

struct Pix {
    capture: CaptureNextFrame,
    out: Vec<u16>,
}

static PIX: OnceLock<Option<Pix>> = OnceLock::new();

fn number(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

/// Loads the capturer when `VICTORY_PIX` names its DLL by full path.
/// Called before the host device is made.
pub(crate) fn load() {
    PIX.get_or_init(|| {
        let path = std::path::PathBuf::from(std::env::var_os("VICTORY_PIX")?);
        if !path.is_absolute() || !path.is_file() {
            tracing::warn!(path = %path.display(), "VICTORY_PIX: not the full path of a file; PIX is not loaded");
            return None;
        }
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::LibraryLoader::{
            GetProcAddress, LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW,
        };
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: a NUL-terminated full path, named by the user.
        let module =
            unsafe { LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), LOAD_WITH_ALTERED_SEARCH_PATH) };
        if module.is_null() {
            tracing::warn!(path = %path.display(), "VICTORY_PIX: the DLL does not load");
            return None;
        }
        // SAFETY: a loaded module and a NUL-terminated name.
        let Some(entry) = (unsafe { GetProcAddress(module, c"CaptureNextFrame".as_ptr().cast()) }) else {
            tracing::warn!("VICTORY_PIX: the DLL has no CaptureNextFrame");
            return None;
        };
        // SAFETY: the export's signature (pix3_win.h).
        let capture = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, CaptureNextFrame>(entry) };
        let out = std::env::var_os("VICTORY_PIX_OUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("victory_pix").join("frame.wpix"));
        if let Some(folder) = out.parent() {
            let _ = std::fs::create_dir_all(folder);
        }
        tracing::info!(
            path = %path.display(),
            out = %out.display(),
            frame = number("VICTORY_PIX_FRAME", 2000),
            "PIX loaded: it captures the frames after the one named"
        );
        Some(Pix { capture, out: out.as_os_str().encode_wide().chain([0]).collect() })
    });
}

/// At the present of `frame`.
pub(crate) fn frame_presented(frame: u64) {
    let Some(pix) = PIX.get().and_then(|p| p.as_ref()) else {
        return;
    };
    if frame == number("VICTORY_PIX_FRAME", 2000) {
        let frames = number("VICTORY_PIX_FRAMES", 1).clamp(1, 8) as u32;
        // SAFETY: the export's signature; a NUL-terminated path it copies.
        let hr = unsafe { (pix.capture)(pix.out.as_ptr(), frames) };
        tracing::info!(frame, frames, hr = format_args!("{hr:#x}"), "PIX is asked for the next frames");
    }
}
