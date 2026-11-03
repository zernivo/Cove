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
            let px = match p.strip_prefix("hwnd:") {
                Some(h) => h.parse::<isize>().ok().and_then(imp::window_icon),
                None => imp::render(&p, SIZE),
            };
            store(p, px.and_then(|(w, h, rgba)| encode(w, h, &rgba)));
        }
    });
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
    #[test]
    fn encodes_png() {
        let png = super::encode(1, 1, &[255, 0, 0, 255]).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        super::store("t".into(), Some(png.clone()));
        let u = super::get("t").unwrap();
        assert_eq!(super::serve(&u[u.rfind('/').unwrap()..]).unwrap(), png);
    }
}
