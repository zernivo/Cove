//! Real app icons for the dock, Search and the switcher, via the documented
//! Shell API (IShellItemImageFactory), at full resolution (256 px) so they
//! stay sharp when the dock magnifies them. Rendered once in the background
//! and served to the UI from memory through the `coveicon` URL scheme, so a
//! dock refresh never re-sends image data.
//!
//! A key is a file path, "shell:AppsFolder\\<AppID>" (Store and Start apps),
//! or "hwnd:<n>" (the window's own icon, for apps Cove can't open).

use std::collections::HashMap;
use std::sync::Mutex;

pub const SIZE: i32 = 256;

struct Cache {
    ids: HashMap<String, Option<usize>>,
    pngs: Vec<Vec<u8>>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(|| Cache { ids: HashMap::new(), pngs: Vec::new() }))
}

/// URL of a rendered icon, if it's ready.
pub fn get(path: &str) -> Option<String> {
    with(|c| c.ids.get(path).copied().flatten()).map(url)
}

/// Has this key been tried (whether or not it produced an icon)?
pub fn known(path: &str) -> bool {
    with(|c| c.ids.contains_key(path))
}

fn url(id: usize) -> String {
    // WebView2 serves custom schemes as http://<scheme>.localhost/.
    if cfg!(windows) { format!("http://coveicon.localhost/{id}.png") } else { format!("coveicon://localhost/{id}.png") }
}

/// PNG bytes for a `coveicon` request path such as "/12.png".
pub fn serve(path: &str) -> Option<Vec<u8>> {
    let id: usize = path.trim_start_matches('/').trim_end_matches(".png").parse().ok()?;
    with(|c| c.pngs.get(id).cloned())
}

fn store(key: String, png: Option<Vec<u8>>) {
    with(|c| {
        let id = png.map(|b| {
            c.pngs.push(b);
            c.pngs.len() - 1
        });
        c.ids.insert(key, id);
    });
}

/// Render any paths not cached yet (call off the UI thread).
pub fn fill(paths: Vec<String>) {
    let mut todo: Vec<String> = paths.into_iter().filter(|p| !known(p)).collect();
    todo.dedup();
    if todo.is_empty() {
        return;
    }
    imp::with_com(|| {
        for p in todo {
            if known(&p) {
                continue;
            }
            let px = if let Some(h) = p.strip_prefix("hwnd:") {
                h.parse::<isize>().ok().and_then(imp::window_icon)
            } else if let Some(r) = p.strip_prefix("res:") {
                // "file,index": the file itself renders as its first icon.
                let file = r.rsplit_once(',').map(|(f, _)| f).unwrap_or(r).trim_matches('"');
                imp::render(&std::path::PathBuf::from(file).to_string_lossy(), SIZE)
            } else {
                imp::render(&p, SIZE)
            };
            store(p, px.map(|(w, h, rgba)| normalise(w, h, rgba)).and_then(|(w, h, rgba)| encode(w, h, &rgba)));
        }
    });
}

/// Below this share of the canvas an icon's artwork is padded out (Store
/// apps' unplated icons carry a wide transparent margin; most desktop app
/// icons fill ~85-95%), and is grown to `FILL`, so icons look one size in
/// the dock, Search and the switcher.
const SMALL: f32 = 0.74;
const FILL: f32 = 0.86;

/// Make an icon's visible artwork fill a consistent share of a square
/// canvas. Icons that already fill it are left exactly as they are.
pub fn normalise(w: u32, h: u32, rgba: Vec<u8>) -> (u32, u32, Vec<u8>) {
    let (w_, h_) = (w as usize, h as usize);
    if w_ == 0 || h_ == 0 || rgba.len() < w_ * h_ * 4 {
        return (w, h, rgba);
    }
    // Bounding box of what you can see.
    let (mut x0, mut y0, mut x1, mut y1) = (w_, h_, 0usize, 0usize);
    for y in 0..h_ {
        for x in 0..w_ {
            if rgba[(y * w_ + x) * 4 + 3] > 12 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return (w, h, rgba); // fully transparent: nothing to measure
    }
    let side = w_.max(h_) as f32;
    let (bw, bh) = ((x1 - x0) as f32, (y1 - y0) as f32);
    if bw.max(bh) / side >= SMALL {
        return (w, h, rgba);
    }
    let out = w_.max(h_);
    let k = FILL * side / bw.max(bh);
    let (nw, nh) = (bw * k, bh * k);
    let (ox, oy) = ((out as f32 - nw) / 2.0, (out as f32 - nh) / 2.0);
    let mut dst = vec![0u8; out * out * 4];
    // Bilinear on premultiplied colour, so edges don't pick up dark fringes.
    let px = |x: isize, y: isize| -> [f32; 4] {
        if x < 0 || y < 0 || x >= w_ as isize || y >= h_ as isize {
            return [0.0; 4];
        }
        let i = (y as usize * w_ + x as usize) * 4;
        let a = rgba[i + 3] as f32 / 255.0;
        [rgba[i] as f32 * a, rgba[i + 1] as f32 * a, rgba[i + 2] as f32 * a, a]
    };
    for y in 0..out {
        for x in 0..out {
            let sx = x0 as f32 + (x as f32 + 0.5 - ox) / k - 0.5;
            let sy = y0 as f32 + (y as f32 + 0.5 - oy) / k - 0.5;
            if sx < x0 as f32 - 1.0 || sy < y0 as f32 - 1.0 || sx > x1 as f32 || sy > y1 as f32 {
                continue;
            }
            let (fx, fy) = (sx.floor(), sy.floor());
            let (tx, ty) = (sx - fx, sy - fy);
            let (ix, iy) = (fx as isize, fy as isize);
            let (a, b, c, d) = (px(ix, iy), px(ix + 1, iy), px(ix, iy + 1), px(ix + 1, iy + 1));
            let mut v = [0f32; 4];
            for j in 0..4 {
                v[j] = (a[j] * (1.0 - tx) + b[j] * tx) * (1.0 - ty) + (c[j] * (1.0 - tx) + d[j] * tx) * ty;
            }
            let o = (y * out + x) * 4;
            if v[3] > 0.0 {
                for j in 0..3 {
                    dst[o + j] = (v[j] / v[3]).round().clamp(0.0, 255.0) as u8;
                }
                dst[o + 3] = (v[3] * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (out as u32, out as u32, dst)
}

fn encode(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut buf, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(rgba).ok()?;
    }
    Some(buf)
}

#[cfg(windows)]
mod imp {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY};
    use windows::Win32::Graphics::Gdi::HBITMAP;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassLongPtrW, GetIconInfo, SendMessageTimeoutW, GCLP_HICON, HICON, ICONINFO, ICON_BIG, SMTO_ABORTIFHUNG, WM_GETICON,
    };
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

    pub fn with_com(f: impl FnOnce()) {
        let ok = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        f();
        if ok {
            unsafe { CoUninitialize() };
        }
    }

    /// Path may be a file path or "shell:AppsFolder\\<AppID>".
    pub fn render(path: &str, size: i32) -> Option<(u32, u32, Vec<u8>)> {
        unsafe {
            let factory: IShellItemImageFactory =
                SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
            let hbmp = factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK).ok()?;
            pixels(hbmp, true)
        }
    }

    /// The icon a window shows in its own title bar (fallback for apps
    /// whose exe Cove can't read, such as ones running as admin).
    pub fn window_icon(h: isize) -> Option<(u32, u32, Vec<u8>)> {
        unsafe {
            let hwnd = HWND(h as *mut _);
            let mut res = 0usize;
            let _ = SendMessageTimeoutW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), LPARAM(0), SMTO_ABORTIFHUNG, 200, Some(&mut res));
            let mut hi = res as isize;
            if hi == 0 {
                hi = GetClassLongPtrW(hwnd, GCLP_HICON) as isize;
            }
            if hi == 0 {
                return None;
            }
            let mut ii = ICONINFO::default();
            GetIconInfo(HICON(hi as *mut _), &mut ii).ok()?;
            let out = pixels(ii.hbmColor, true);
            if !ii.hbmMask.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));
            }
            out
        }
    }

    /// Read a 32-bit bitmap into straight-alpha RGBA, then free it.
    unsafe fn pixels(hbmp: HBITMAP, free: bool) -> Option<(u32, u32, Vec<u8>)> {
        unsafe {
            if hbmp.is_invalid() {
                return None;
            }
            let obj = HGDIOBJ(hbmp.0);
            let mut bm = BITMAP::default();
            if GetObjectW(obj, std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) == 0 {
                let _ = DeleteObject(obj);
                return None;
            }
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            let mut info = BITMAPINFO::default();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };
            let mut px = vec![0u8; (w * h * 4) as usize];
            let dc = CreateCompatibleDC(None);
            let lines = GetDIBits(dc, hbmp, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
            let _ = DeleteDC(dc);
            if free {
                let _ = DeleteObject(obj);
            }
            if lines == 0 {
                return None;
            }
            // BGRA premultiplied -> RGBA straight
            let any_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
            for p in px.chunks_exact_mut(4) {
                let a = if any_alpha { p[3] } else { 255 };
                let (b, g, r) = (p[0], p[1], p[2]);
                let un = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8 };
                p[0] = un(r);
                p[1] = un(g);
                p[2] = un(b);
                p[3] = a;
            }
            Some((w as u32, h as u32, px))
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn with_com(f: impl FnOnce()) {
        f()
    }
    pub fn render(_path: &str, _size: i32) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
    pub fn window_icon(_h: isize) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
}

pub use imp::with_com;

#[cfg(test)]
mod tests {
    fn square(canvas: u32, from: u32, to: u32) -> Vec<u8> {
        let mut v = vec![0u8; (canvas * canvas * 4) as usize];
        for y in from..to {
            for x in from..to {
                let i = ((y * canvas + x) * 4) as usize;
                v[i..i + 4].copy_from_slice(&[200, 100, 50, 255]);
            }
        }
        v
    }
    fn extent(w: u32, v: &[u8]) -> u32 {
        let rows: Vec<u32> = (0..w).filter(|y| (0..w).any(|x| v[((y * w + x) * 4 + 3) as usize] > 12)).collect();
        rows.last().unwrap() - rows[0] + 1
    }
    #[test]
    fn small_artwork_is_grown() {
        // Store-style: 128 px of art in a 256 canvas (50%) -> ~86%.
        let (w, h, v) = super::normalise(256, 256, square(256, 64, 192));
        assert_eq!((w, h), (256, 256));
        let e = extent(256, &v);
        assert!((216..=226).contains(&e), "extent {e}");
        // Colour survives the resample.
        let mid = ((128 * 256 + 128) * 4) as usize;
        assert_eq!(&v[mid..mid + 4], &[200, 100, 50, 255]);
    }
    #[test]
    fn full_icons_are_untouched() {
        let src = square(256, 10, 246);
        let (_, _, v) = super::normalise(256, 256, src.clone());
        assert_eq!(v, src);
    }

    #[test]
    fn encodes_png() {
        let png = super::encode(1, 1, &[255, 0, 0, 255]).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        super::store("t".into(), Some(png.clone()));
        let u = super::get("t").unwrap();
        assert_eq!(super::serve(&u[u.rfind('/').unwrap()..]).unwrap(), png);
    }
}
