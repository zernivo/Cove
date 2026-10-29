//! Real icons for Cove Search results, via the documented Shell API
//! (IShellItemImageFactory). Rendered once in the background, cached as
//! small PNG data URLs.

use std::collections::HashMap;
use std::sync::Mutex;

static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

pub fn get(path: &str) -> Option<String> {
    CACHE.lock().unwrap().as_ref()?.get(path).cloned().flatten()
}

/// Render any paths not cached yet (call off the UI thread).
pub fn fill(paths: Vec<String>) {
    let todo: Vec<String> = {
        let mut g = CACHE.lock().unwrap();
        let map = g.get_or_insert_with(HashMap::new);
        paths.into_iter().filter(|p| !map.contains_key(p)).collect()
    };
    if todo.is_empty() {
        return;
    }
    imp::with_com(|| {
        for p in todo {
            let icon = imp::render(&p, 48).and_then(|(w, h, rgba)| encode(w, h, &rgba));
            CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(p, icon);
        }
    });
}

fn encode(w: u32, h: u32, rgba: &[u8]) -> Option<String> {
    use base64::Engine;
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut buf, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(rgba).ok()?;
    }
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(buf)))
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
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY};

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
            let hbmp = factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY).ok()?;
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
            let _ = DeleteObject(obj);
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_png() {
        let s = super::encode(1, 1, &[255, 0, 0, 255]).unwrap();
        assert!(s.starts_with("data:image/png;base64,"));
    }
}
