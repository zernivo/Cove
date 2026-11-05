//! Real app icons for the dock, Search and the switcher, via the documented
//! Shell API (IShellItemImageFactory), at full resolution (256 px) so they
//! stay sharp when the dock magnifies them. Rendered once in the background
//! and served to the UI from memory through the `coveicon` URL scheme, so a
//! dock refresh never re-sends image data.
//!
//! A key is a file path, "shell:AppsFolder\\<AppID>" (Store and Start apps),
//! "hwnd:<n>" (the window's own icon, for apps Cove can't open),
//! "pkglogo:<Family!AppId>" (a packaged app's own logo PNG, the largest
//! unplated target size, read via its AppxManifest), or
//! "stock:recycler" / "stock:recyclerfull" (Windows' stock bin icons).
//! Anything that can't be drawn is logged (cove.log) and the caller falls
//! back to the next key, ending at the letter tile.

use std::collections::HashMap;
use std::sync::Mutex;

pub const SIZE: i32 = 256;

struct Cache {
    ids: HashMap<String, Option<usize>>,
    pngs: Vec<Vec<u8>>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

type Logger = Box<dyn Fn(&str) + Send + Sync>;
static LOG: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

/// Where icon failures are reported (cove.log).
pub fn set_logger(f: impl Fn(&str) + Send + Sync + 'static) {
    let _ = LOG.set(Box::new(f));
}
fn log(msg: &str) {
    if let Some(f) = LOG.get() {
        f(msg);
    }
}

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
            } else if let Some(exe) = p.strip_prefix("exeicon:") {
                imp::exe_icon(exe)
            } else if p == "stock:recycler" {
                imp::stock_icon(31) // SIID_RECYCLER
            } else if p == "stock:recyclerfull" {
                imp::stock_icon(32) // SIID_RECYCLERFULL
            } else if let Some(aumid) = p.strip_prefix("pkglogo:") {
                package_logo(aumid)
            } else if let Some(r) = p.strip_prefix("res:") {
                // "file,index": the file itself renders as its first icon.
                let file = r.rsplit_once(',').map(|(f, _)| f).unwrap_or(r).trim_matches('"');
                imp::render(&std::path::PathBuf::from(file).to_string_lossy(), SIZE)
            } else {
                imp::render(&p, SIZE)
            };
            if px.is_none() && (p.starts_with("pkglogo:") || p.starts_with("stock:") || p.starts_with("shell:") || p.starts_with("::")) {
                log(&format!("icons: no picture for {p}; falling back to the next source"));
            }
            store(p, px.map(|(w, h, rgba)| shrink(w, h, rgba, SIZE as u32)).map(|(w, h, rgba)| normalise(w, h, rgba)).and_then(|(w, h, rgba)| encode(w, h, &rgba)));
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

// ---- Packaged apps' own logos (Settings, Store apps, MSIX) ----

/// The logo an app's AppxManifest gives it: Square44x44Logo (the app-list
/// icon, which ships unplated target sizes), else Square150x150Logo, else
/// the package Logo. Relative to the package root, backslashes as written.
pub fn manifest_logo(xml: &str, app_id: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let want = format!("id=\"{}\"", app_id.to_ascii_lowercase());
    // The <Application ...> element for this app id, up to its end tag.
    let mut from = 0;
    let block = loop {
        let at = lower[from..].find("<application ")? + from;
        let end = lower[at..].find("</application>").map(|e| at + e).unwrap_or(lower.len());
        let head_end = lower[at..].find('>').map(|e| at + e).unwrap_or(end);
        if lower[at..head_end].contains(&want) {
            break (at, end);
        }
        from = end.max(at + 1);
    };
    let attr = |name: &str| -> Option<String> {
        let key = format!("{}=\"", name.to_ascii_lowercase());
        let i = lower[block.0..block.1].find(&key)? + block.0 + key.len();
        let j = xml[i..].find('"')? + i;
        Some(xml[i..j].to_string()).filter(|v| !v.is_empty())
    };
    attr("Square44x44Logo").or_else(|| attr("Square150x150Logo")).or_else(|| {
        let i = lower.find("<logo>")? + 6;
        let j = lower[i..].find("</logo>")? + i;
        Some(xml[i..j].trim().to_string())
    })
}

/// Pick the sharpest file for `logo` ("Assets\\AppList.png") among the
/// files in its folder: the largest target size up to 256 px, unplated
/// preferred, then the largest scale. High-contrast and light-theme
/// variants are skipped unless nothing else exists.
pub fn best_logo(files: &[String], logo: &str) -> Option<String> {
    let name = logo.rsplit(['\\', '/']).next()?;
    let (stem, ext) = name.rsplit_once('.')?;
    let (stem, ext) = (stem.to_ascii_lowercase(), ext.to_ascii_lowercase());
    let base: u32 = if logo.to_ascii_lowercase().contains("150") { 150 } else { 44 };
    let mut best: Option<((u32, u32, i64, u32), String)> = None;
    for f in files {
        let fl = f.to_ascii_lowercase();
        let Some(rest) = fl.strip_prefix(&format!("{stem}.")) else { continue };
        let Some(quals) = rest.strip_suffix(&format!(".{ext}")).map(Some).unwrap_or_else(|| (rest == ext).then_some("")) else { continue };
        let q: Vec<&str> = quals.split('_').filter(|s| !s.is_empty()).collect();
        let odd = q.iter().any(|x| x.starts_with("contrast-") || *x == "altform-lightunplated" || x.starts_with("theme-light"));
        let unplated = q.iter().any(|x| *x == "altform-unplated");
        let px = q.iter().find_map(|x| x.strip_prefix("targetsize-").and_then(|n| n.parse::<u32>().ok()))
            .or_else(|| q.iter().find_map(|x| x.strip_prefix("scale-").and_then(|n| n.parse::<u32>().ok())).map(|sc| base * sc / 100))
            .unwrap_or(base);
        let key = (u32::from(!odd), px.min(256), -(px as i64 - 256).max(0), u32::from(unplated));
        if best.as_ref().map_or(true, |(k, _)| key > *k) {
            best = Some((key, f.clone()));
        }
    }
    best.map(|(_, f)| f)
}

/// A packaged app's logo as RGBA, via its install folder and manifest.
fn package_logo(aumid: &str) -> Option<(u32, u32, Vec<u8>)> {
    let (family, app_id) = aumid.split_once('!')?;
    let root = imp::package_root(family)?;
    let xml = std::fs::read_to_string(root.join("AppxManifest.xml")).ok()?;
    let logo = manifest_logo(&xml, app_id)?;
    let rel = logo.replace('\\', std::path::MAIN_SEPARATOR_STR);
    let dir = root.join(&rel).parent()?.to_path_buf();
    let files: Vec<String> = std::fs::read_dir(&dir).ok()?.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect();
    let pick = best_logo(&files, &logo)?;
    decode_png(&std::fs::read(dir.join(pick)).ok()?)
}

/// Any PNG as straight RGBA 8-bit.
pub fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().ok()?;
    let mut buf = vec![0u8; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == (w * h * 4) as usize).then_some((w, h, rgba))
}

/// Box-filter a picture down so its longer side is at most `max` px.
pub fn shrink(w: u32, h: u32, rgba: Vec<u8>, max: u32) -> (u32, u32, Vec<u8>) {
    if w <= max && h <= max || w == 0 || h == 0 {
        return (w, h, rgba);
    }
    let k = w.max(h) as f32 / max as f32;
    let (nw, nh) = (((w as f32 / k).round() as u32).max(1), ((h as f32 / k).round() as u32).max(1));
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let (x0, x1) = ((x as f32 * k) as u32, (((x + 1) as f32 * k) as u32).min(w).max((x as f32 * k) as u32 + 1));
            let (y0, y1) = ((y as f32 * k) as u32, (((y + 1) as f32 * k) as u32).min(h).max((y as f32 * k) as u32 + 1));
            let mut acc = [0f32; 4];
            let mut n = 0f32;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = ((sy * w + sx) * 4) as usize;
                    let a = rgba[i + 3] as f32 / 255.0;
                    acc[0] += rgba[i] as f32 * a;
                    acc[1] += rgba[i + 1] as f32 * a;
                    acc[2] += rgba[i + 2] as f32 * a;
                    acc[3] += a;
                    n += 1.0;
                }
            }
            let o = ((y * nw + x) * 4) as usize;
            if acc[3] > 0.0 {
                out[o] = (acc[0] / acc[3]).round().min(255.0) as u8;
                out[o + 1] = (acc[1] / acc[3]).round().min(255.0) as u8;
                out[o + 2] = (acc[2] / acc[3]).round().min(255.0) as u8;
            }
            out[o + 3] = (acc[3] / n * 255.0).round().min(255.0) as u8;
        }
    }
    (nw, nh, out)
}

/// Named system icons for menus, Search and Control Centre: the key to
/// draw each from (first that works). Rendered on demand like any icon.
pub fn system_keys() -> Vec<(&'static str, Vec<String>)> {
    let win = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let known = |name: &str| format!(r"{home}\{name}");
    vec![
        ("settings", vec!["pkglogo:windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel".into(), r"shell:AppsFolder\windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel".into()]),
        ("explorer", vec![r"shell:AppsFolder\Microsoft.Windows.Explorer".into(), format!(r"exeicon:{win}\explorer.exe")]),
        ("taskmgr", vec![format!(r"exeicon:{win}\System32\Taskmgr.exe"), format!(r"{win}\System32\Taskmgr.exe")]),
        ("controlpanel", vec!["::{26EE0668-A00A-44D7-9371-BEB064C98683}".into(), "shell:ControlPanelFolder".into()]),
        ("thispc", vec!["::{20D04FE0-3AEA-1069-A2D8-08002B30309D}".into(), "shell:MyComputerFolder".into()]),
        ("downloads", vec!["shell:Downloads".into(), known("Downloads")]),
        ("documents", vec!["shell:Personal".into(), known("Documents")]),
        ("desktop", vec!["shell:Desktop".into(), known("Desktop")]),
        ("pictures", vec!["shell:My Pictures".into(), known("Pictures")]),
        ("home", vec![home.clone()]),
        ("recycler", vec!["stock:recycler".into(), "::{645FF040-5081-101B-9F08-08002B2CF60E}".into()]),
        ("recyclerfull", vec!["stock:recyclerfull".into()]),
    ]
}

/// Another app's tray icon (an HICON) as PNG, for tray.rs.
#[cfg(windows)]
pub fn hicon_png(hicon: isize) -> Option<Vec<u8>> {
    let (w, h, rgba) = imp::hicon_rgba(hicon)?;
    encode(w, h, &rgba)
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

    /// The icon a window shows (fallback for apps whose exe Cove can't
    /// read, such as ones running as admin, and Electron apps whose shell
    /// item has no picture): WM_GETICON big, then small, then the class
    /// icons. These belong to the app: never destroyed here.
    pub fn window_icon(h: isize) -> Option<(u32, u32, Vec<u8>)> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetClassLongPtrW, SendMessageTimeoutW, GCLP_HICON, GCLP_HICONSM, ICON_BIG, ICON_SMALL, ICON_SMALL2, SMTO_ABORTIFHUNG,
            WM_GETICON,
        };
        unsafe {
            let hwnd = h as windows_sys::Win32::Foundation::HWND;
            for which in [ICON_BIG, ICON_SMALL2, ICON_SMALL] {
                let mut res = 0usize;
                SendMessageTimeoutW(hwnd, WM_GETICON, which as usize, 0, SMTO_ABORTIFHUNG, 200, &mut res);
                if res != 0 {
                    if let Some(px) = hicon_rgba(res as isize) {
                        return Some(px);
                    }
                }
            }
            for which in [GCLP_HICON, GCLP_HICONSM] {
                let hi = GetClassLongPtrW(hwnd, which) as isize;
                if hi != 0 {
                    if let Some(px) = hicon_rgba(hi) {
                        return Some(px);
                    }
                }
            }
            None
        }
    }

    /// The exe's own icon resource at full size (PrivateExtractIcons), then
    /// the shell's file icon (SHGetFileInfo).
    pub fn exe_icon(exe: &str) -> Option<(u32, u32, Vec<u8>)> {
        resource_icon(exe, 0).or_else(|| file_info_icon(exe))
    }

    /// A stock shell icon (SIID_*) as Windows itself draws it, at 256 px.
    pub fn stock_icon(siid: i32) -> Option<(u32, u32, Vec<u8>)> {
        use windows_sys::Win32::UI::Shell::{SHGetStockIconInfo, SHSTOCKICONINFO};
        unsafe {
            let mut si: SHSTOCKICONINFO = std::mem::zeroed();
            si.cbSize = std::mem::size_of::<SHSTOCKICONINFO>() as u32;
            const SHGSI_ICONLOCATION: u32 = 0;
            if SHGetStockIconInfo(siid as _, SHGSI_ICONLOCATION as _, &mut si) < 0 {
                return None;
            }
            let n = si.szPath.iter().position(|&c| c == 0).unwrap_or(si.szPath.len());
            let path = String::from_utf16_lossy(&si.szPath[..n]);
            resource_icon(&path, si.iIcon)
        }
    }

    /// Where a package family is installed: its newest registered package's
    /// PackageRootFolder (the AppModel repository in HKCU, read-only).
    pub fn package_root(family: &str) -> Option<std::path::PathBuf> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let (name, publisher) = family.rsplit_once('_')?;
        let repo = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages")
            .ok();
        let head = format!("{}_", name.to_ascii_lowercase());
        let tail = format!("_{}", publisher.to_ascii_lowercase());
        let mut found: Vec<(String, std::path::PathBuf)> = Vec::new();
        if let Some(repo) = repo {
            for k in repo.enum_keys().flatten() {
                let kl = k.to_ascii_lowercase();
                if kl.starts_with(&head) && kl.ends_with(&tail) {
                    if let Ok(root) = repo.open_subkey(&k).and_then(|s| s.get_value::<String, _>("PackageRootFolder")) {
                        let p = std::path::PathBuf::from(root);
                        if p.join("AppxManifest.xml").is_file() {
                            found.push((kl, p));
                        }
                    }
                }
            }
        }
        found.sort();
        if let Some((_, p)) = found.pop() {
            return Some(p);
        }
        // System apps that live in a fixed folder.
        let win = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let fixed = match name.to_ascii_lowercase().as_str() {
            "windows.immersivecontrolpanel" => Some(format!(r"{win}\ImmersiveControlPanel")),
            _ => None,
        }?;
        let p = std::path::PathBuf::from(fixed);
        p.join("AppxManifest.xml").is_file().then_some(p)
    }

    /// Icon `index` (or resource id when negative) of a PE file at 256 px.
    fn resource_icon(file: &str, index: i32) -> Option<(u32, u32, Vec<u8>)> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, PrivateExtractIconsW};
        let w: Vec<u16> = file.encode_utf16().chain(std::iter::once(0)).collect();
        if w.len() > 260 {
            return None;
        }
        unsafe {
            let mut hi: windows_sys::Win32::UI::WindowsAndMessaging::HICON = std::ptr::null_mut();
            let mut id = 0u32;
            let got = PrivateExtractIconsW(w.as_ptr(), index, super::SIZE, super::SIZE, &mut hi, &mut id, 1, 0);
            if got == 0 || got == u32::MAX || hi.is_null() {
                return None;
            }
            let px = hicon_rgba(hi as isize);
            DestroyIcon(hi);
            px
        }
    }

    fn file_info_icon(file: &str) -> Option<(u32, u32, Vec<u8>)> {
        use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
        use windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon;
        let w: Vec<u16> = file.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let mut fi: SHFILEINFOW = std::mem::zeroed();
            let ok = SHGetFileInfoW(w.as_ptr(), 0, &mut fi, std::mem::size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_LARGEICON);
            if ok == 0 || fi.hIcon.is_null() {
                return None;
            }
            let px = hicon_rgba(fi.hIcon as isize);
            DestroyIcon(fi.hIcon);
            px
        }
    }

    /// An HICON as straight-alpha RGBA. Old icons without an alpha channel
    /// take their transparency from the icon's mask.
    pub(super) fn hicon_rgba(hicon: isize) -> Option<(u32, u32, Vec<u8>)> {
        use windows_sys::Win32::Graphics::Gdi::DeleteObject;
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
        unsafe {
            let mut ii: ICONINFO = std::mem::zeroed();
            if GetIconInfo(hicon as _, &mut ii) == 0 {
                return None;
            }
            let color = HBITMAP(ii.hbmColor as *mut _);
            let mask = ii.hbmMask;
            let out = if ii.hbmColor.is_null() {
                None
            } else {
                let had_alpha = has_alpha(color);
                let px = pixels(color, true);
                match (px, had_alpha) {
                    (Some((w, h, mut rgba)), false) if !mask.is_null() => {
                        if let Some((mw, _mh, m)) = pixels(HBITMAP(mask as *mut _), false) {
                            if mw == w {
                                for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
                                    // Mask: black = opaque, white = transparent.
                                    if m.get(i * 4).is_some_and(|&v| v > 127) {
                                        p[3] = 0;
                                    }
                                }
                            }
                        }
                        Some((w, h, rgba))
                    }
                    (px, _) => px,
                }
            };
            if !mask.is_null() {
                DeleteObject(mask);
            }
            out.filter(|(w, h, _)| *w > 0 && *h > 0)
        }
    }

    /// Does a 32-bit bitmap carry any alpha (checked without freeing it)?
    unsafe fn has_alpha(hbmp: HBITMAP) -> bool {
        unsafe {
            let obj = HGDIOBJ(hbmp.0);
            let mut bm = BITMAP::default();
            if GetObjectW(obj, std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) == 0 {
                return false;
            }
            let (w, h) = (bm.bmWidth, bm.bmHeight);
            let mut info = BITMAPINFO::default();
            info.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() };
            let mut px = vec![0u8; (w * h * 4).max(0) as usize];
            let dc = CreateCompatibleDC(None);
            let lines = GetDIBits(dc, hbmp, 0, h as u32, Some(px.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
            let _ = DeleteDC(dc);
            lines != 0 && px.chunks_exact(4).any(|p| p[3] != 0)
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
    pub fn exe_icon(_exe: &str) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
    pub fn stock_icon(_siid: i32) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
    pub fn package_root(_family: &str) -> Option<std::path::PathBuf> {
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
    fn reads_the_app_list_logo_from_a_manifest() {
        let xml = r#"<Package><Properties><Logo>Images\StoreLogo.png</Logo></Properties><Applications>
            <Application Id="Other"><uap:VisualElements Square44x44Logo="Other.png"/></Application>
            <Application Id="microsoft.windows.immersivecontrolpanel" Executable="SystemSettings.exe">
              <uap:VisualElements DisplayName="Settings" Square150x150Logo="Images\Logo.png" Square44x44Logo="Images\SmallLogo.png"/>
            </Application></Applications></Package>"#;
        assert_eq!(super::manifest_logo(xml, "microsoft.windows.immersivecontrolpanel").as_deref(), Some(r"Images\SmallLogo.png"));
        assert_eq!(super::manifest_logo(xml, "Other").as_deref(), Some("Other.png"));
        assert_eq!(super::manifest_logo(xml, "Missing"), None);
    }
    #[test]
    fn picks_the_sharpest_unplated_logo() {
        let files: Vec<String> = [
            "SmallLogo.png", "SmallLogo.scale-200.png", "SmallLogo.targetsize-48.png", "SmallLogo.targetsize-256.png",
            "SmallLogo.targetsize-256_altform-unplated.png", "SmallLogo.targetsize-256_altform-lightunplated.png",
            "SmallLogo.contrast-black_targetsize-256.png", "Logo.scale-400.png",
        ].iter().map(|s| s.to_string()).collect();
        assert_eq!(super::best_logo(&files, r"Images\SmallLogo.png").as_deref(), Some("SmallLogo.targetsize-256_altform-unplated.png"));
        let few: Vec<String> = ["SmallLogo.scale-100.png", "SmallLogo.scale-400.png"].iter().map(|s| s.to_string()).collect();
        assert_eq!(super::best_logo(&few, r"Images\SmallLogo.png").as_deref(), Some("SmallLogo.scale-400.png"));
        assert_eq!(super::best_logo(&few, r"Images\Other.png"), None);
    }
    #[test]
    fn shrinks_and_decodes() {
        let (w, h, v) = super::shrink(512, 512, vec![255; 512 * 512 * 4], 256);
        assert_eq!((w, h, v.len()), (256, 256, 256 * 256 * 4));
        let png = super::encode(2, 1, &[1, 2, 3, 255, 4, 5, 6, 128]).unwrap();
        assert_eq!(super::decode_png(&png), Some((2, 1, vec![1, 2, 3, 255, 4, 5, 6, 128])));
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
