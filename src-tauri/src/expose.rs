//! App Exposé: every window of one app at a glance, as live previews.
//! Uses documented DWM thumbnails (the same API the taskbar uses for its
//! previews), drawn into Cove's own full-screen overlay. Read-only: no
//! other process is touched.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    pub hwnd: isize,
    pub title: String,
    /// Destination rect in physical pixels, relative to the overlay.
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Minimized {
    pub hwnd: isize,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExposeState {
    pub name: String,
    pub tiles: Vec<Tile>,
    pub minimized: Vec<Minimized>,
    /// Overlay scale factor, so the UI can convert physical rects to CSS pixels.
    pub scale: f64,
}

const TOP: i32 = 96; // room for the app name
const BOTTOM_MIN: i32 = 120; // room for minimized windows
const PAD: i32 = 28;
const CAPTION: i32 = 30;

/// Grid cells for `n` previews inside `w`×`h`, as (x, y, w, h).
pub fn grid(n: usize, w: i32, h: i32) -> Vec<(i32, i32, i32, i32)> {
    if n == 0 || w <= 0 || h <= 0 {
        return Vec::new();
    }
    // Pick the column count that gives the biggest 16:10 cells.
    let mut best = (1usize, 0f64);
    for cols in 1..=n {
        let rows = n.div_ceil(cols);
        let cw = w as f64 / cols as f64;
        let ch = h as f64 / rows as f64;
        let s = (cw / 16.0).min(ch / 10.0);
        if s > best.1 {
            best = (cols, s);
        }
    }
    let cols = best.0;
    let rows = n.div_ceil(cols);
    let (cw, ch) = (w / cols as i32, h / rows as i32);
    (0..n)
        .map(|i| {
            let (r, c) = ((i / cols) as i32, (i % cols) as i32);
            // Centre a short last row.
            let in_row = if r as usize == rows - 1 { n - (rows - 1) * cols } else { cols } as i32;
            let off = (w - in_row * cw) / 2;
            (off + c * cw, r * ch, cw, ch)
        })
        .collect()
}

/// Fit a `sw`×`sh` source inside a cell, keeping its shape, leaving room for padding and a caption.
pub fn fit(cell: (i32, i32, i32, i32), sw: i32, sh: i32) -> (i32, i32, i32, i32) {
    let (cx, cy, cw, ch) = cell;
    let (aw, ah) = ((cw - 2 * PAD).max(40), (ch - 2 * PAD - CAPTION).max(30));
    let (sw, sh) = (sw.max(1) as f64, sh.max(1) as f64);
    let s = (aw as f64 / sw).min(ah as f64 / sh).min(1.0);
    let (w, h) = ((sw * s) as i32, (sh * s) as i32);
    (cx + (cw - w) / 2, cy + PAD + (ah - h) / 2, w, h)
}

pub use imp::{close, open};

#[cfg(windows)]
mod imp {
    use super::*;
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{HWND, RECT, SIZE};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
        DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY,
        DWM_TNP_VISIBLE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsIconic, IsWindow};

    static THUMBS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

    fn title(h: HWND) -> String {
        let mut buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    /// Register live previews of `windows` into the overlay `dest`
    /// (sized `w`×`h` physical pixels).
    pub fn open(dest: isize, name: &str, windows: &[isize], w: i32, h: i32) -> ExposeState {
        close();
        let mut st = ExposeState { name: name.to_string(), tiles: Vec::new(), minimized: Vec::new(), scale: 1.0 };
        let mut live: Vec<(HWND, isize, i32, i32)> = Vec::new();
        for &hw in windows {
            let src = hw as HWND;
            unsafe {
                if IsWindow(src) == 0 {
                    continue;
                }
                if IsIconic(src) != 0 {
                    st.minimized.push(Minimized { hwnd: hw, title: title(src) });
                    continue;
                }
                let mut id: isize = 0;
                if DwmRegisterThumbnail(dest as HWND, src, &mut id) != 0 || id == 0 {
                    continue;
                }
                let mut sz = SIZE { cx: 0, cy: 0 };
                DwmQueryThumbnailSourceSize(id, &mut sz);
                live.push((src, id, sz.cx, sz.cy));
            }
        }
        let bottom = if st.minimized.is_empty() { 40 } else { BOTTOM_MIN };
        let cells = grid(live.len(), w - 80, h - TOP - bottom);
        let mut ids = THUMBS.lock().unwrap();
        for ((src, id, sw, sh), cell) in live.into_iter().zip(cells) {
            let (x, y, tw, th) = fit((cell.0 + 40, cell.1 + TOP, cell.2, cell.3), sw, sh);
            let props = DWM_THUMBNAIL_PROPERTIES {
                dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
                rcDestination: RECT { left: x, top: y, right: x + tw, bottom: y + th },
                rcSource: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                opacity: 255,
                fVisible: 1,
                fSourceClientAreaOnly: 0,
            };
            unsafe { DwmUpdateThumbnailProperties(id, &props) };
            ids.push(id);
            st.tiles.push(Tile { hwnd: src as isize, title: title(src), x, y, w: tw, h: th });
        }
        st
    }

    /// Remove every live preview.
    pub fn close() {
        let mut ids = THUMBS.lock().unwrap();
        for id in ids.drain(..) {
            unsafe { DwmUnregisterThumbnail(id) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn open(_dest: isize, name: &str, _windows: &[isize], _w: i32, _h: i32) -> ExposeState {
        ExposeState { name: name.to_string(), tiles: Vec::new(), minimized: Vec::new(), scale: 1.0 }
    }
    pub fn close() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_fits_all() {
        for n in 1..=12 {
            let g = grid(n, 1800, 900);
            assert_eq!(g.len(), n);
            for (x, y, w, h) in g {
                assert!(x >= 0 && y >= 0 && x + w <= 1800 && y + h <= 900);
            }
        }
    }

    #[test]
    fn fit_keeps_shape() {
        let (_, _, w, h) = fit((0, 0, 800, 600), 1600, 900);
        assert!(((w as f64 / h as f64) - 16.0 / 9.0).abs() < 0.02);
        assert!(w <= 800 && h <= 600);
    }
}
