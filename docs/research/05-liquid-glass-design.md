# 05 — Liquid Glass design research → "Cove Glass" spec

*Research date: 2026-10-29. For Cove, a Windows app with its own calm, macOS-inspired interface. Internal reference only: Cove must not copy Apple's assets, icons (SF Symbols), fonts (SF Pro), wallpapers or trademarks. "Liquid Glass" is Apple's name for its material, so Cove's material is called **Cove Glass**.*

Confidence key: **[Apple]** = Apple's own HIG or WWDC session. **[MS]** = Microsoft Learn. **[3p]** = third-party measurement or analysis (check before relying on it). **[inferred]** = my judgment from the sources.

---

## 1. What Liquid Glass is (the core ideas)

Sources: HIG Materials https://developer.apple.com/design/human-interface-guidelines/materials · WWDC25 "Meet Liquid Glass" https://developer.apple.com/videos/play/wwdc2025/219 (full transcript mirror: https://github.com/kevinhirsch/orwell/blob/main/docs/design/liquid-glass/sources/lg_wwdc_219_transcript.md) · WWDC25 "Get to know the new design system" https://developer.apple.com/videos/play/wwdc2025/356/

### 1.1 Two layers: glass sits on top of content
- Liquid Glass is "a distinct functional layer for controls and navigation elements — like tab bars and sidebars — that floats above the content layer." Content scrolls and shows through underneath it. **[Apple, HIG]**
- **Don't use Liquid Glass in the content layer.** Tables, lists and app backgrounds use *standard materials* (blur, vibrancy, blend modes) instead. The one exception: sliders and toggles briefly turn into glass *while you are using them*. **[Apple, HIG]**
- **Use it sparingly.** Keep it to "the most important functional elements." Too much glass pulls attention from content. **[Apple, HIG]**
- **Never put glass on glass.** Anything sitting *on* glass uses fills, transparency and vibrancy so it "feels like a thin overlay that is part of the material." **[Apple, 219]**
- When the app is at rest (for example right after launch), **content and glass should not overlap**. Move or resize content so they stay apart. **[Apple, 219]**
- Remove any extra backgrounds and borders you added to bars. "Hierarchy should be expressed through layout and grouping" rather than decoration. **[Apple, 356]**

### 1.2 Lensing and refraction (what makes it more than blur)
- "Whereas previous materials scattered light, this new set of materials dynamically bends, shapes, and concentrates light in real time." Lensing (bending what's behind the glass) is "the primary way Liquid Glass visually defines itself." **[Apple, 219]**
- Glass **appears and disappears by turning the lensing up or down, not by fading its opacity**. **[Apple, 219]**
- **Bigger glass looks thicker.** As glass grows (for example a toolbar button opening into a menu), it gets deeper shadows, stronger lensing and refraction, and softer light scattering. **[Apple, 219]**
- How people rebuild it **[3p]** (kube.io https://kube.io/blog/liquid-glass-css-svg; LiquiKit https://liquikit.dev/docs/how-it-works):
  - Model a rounded slab with a curved *bezel* (edge band) and a flat middle. Apply Snell's law at index of refraction (IOR) ≈ 1.5. Turn the result into a displacement map: red channel = x shift, green channel = y shift, 128 = no shift. Feed that map to SVG `feDisplacementMap`.
  - **Only the bezel bends light; the flat middle stays sharp and readable.** A plain linear-gradient map produces a sheared, glitchy look (https://github.com/LeonardSEO/liquid-glass-react).
  - Convex **squircle** profile, `y = ⁴√(1−(1−x)⁴)`, gives the smoothest change from curve to flat. Apple appears to use a convex profile, with a "lip" profile (convex rim, concave middle) for the Switch thumb.
  - 8-bit maps cap displacement at about ±127 px. You can animate the filter `scale` to bring the effect in or out without rebuilding the map. That matches Apple's "modulate lensing instead of fading."
  - Optional chromatic dispersion: displace the R, G and B channels separately for a faint prism fringe (https://github.com/ccl125/liquid-glass). Too loud for Cove (see Don'ts).

### 1.3 Specular highlights and light
- A **highlights layer**: light sources in a virtual environment hit the material, so highlights follow its shape. On some interactions the lights move around the shape and trace its outline. On iPhone they can respond to device motion. **[Apple, 219]**
- The rebuilds treat it as a **rim light**. Edge brightness depends on the angle between the surface normal and a fixed light direction, giving two thin crescents on opposite edges. Some add Schlick's Fresnel term so the rim brightens at glancing angles. **[3p: kube.io, LiquiKit, https://github.com/xanaxknight/liquid-glass-css-svg]**
- **Interaction glow:** when pressed, the material "illuminates from within." The glow starts under the pointer, spreads through the element, and reaches nearby glass. **[Apple, 219]**
- On large surfaces like sidebars, **colour from nearby content spills onto the glass and into its shadow**. **[Apple, 219]**

### 1.4 Shadows that adapt
- The shadow gets **darker over text and lighter over plain light backgrounds**, so separation stays even. **[Apple, 219]**

### 1.5 Adaptive tint and legibility
- Every layer adjusts to what's behind it. As text scrolls under, shadows grow, and tint and dynamic range shift so controls stay readable while letting as much content through as possible. **[Apple, 219]**
- **Small elements** (nav bars, tab bars) **switch between light and dark** depending on the background. **Large elements** (menus, sidebars) **don't switch**, because a big area changing would be distracting. Glyphs on glass switch with the glass. **[Apple, 219]**
- **Tinting:** you pick one colour and the system generates a range of tones mapped to the brightness underneath, "like colored glass." Use it **only for primary actions**. If everything is tinted, nothing stands out. Put brand colour in the content layer, not the toolbar. A solid opaque fill "breaks the visual character" of the glass. **[Apple, 219; Meet with Apple https://developer.apple.com/videos/play/meet-with-apple/201/]**
- When focus is lost (inactive window on Mac/iPad), the glass **recedes**. **[Apple, 219]**

### 1.6 Regular vs Clear
| | **Regular** | **Clear** |
|---|---|---|
| Behaviour | Fully adaptive: blurs and adjusts the brightness of what's behind; works at any size, over any content | Not adaptive; always very transparent |
| Use for | Almost everything: alerts, sidebars, popovers, anything with a lot of text | Only over media (photos, video) **and** where a dimming layer won't hurt the content **and** the content on top is bold and bright |
| Legibility aid | Built-in, plus scroll edge effects | Needs a dimming layer. HIG: **~35% dark dim** over bright content; small elements can dim just their own area |
| Rule | **Never mix the two variants** | |
Sources: **[Apple, HIG + 219]**

### 1.7 Concentric shapes
- **Three shape types [Apple, 356]:**
  - **Fixed**: a constant radius.
  - **Capsule**: radius = height ÷ 2.
  - **Concentric**: radius = parent radius − padding.
- Watch for corners that look "pinched" or "flared." Use **concentric with a fallback radius** for parts that are sometimes nested and sometimes stand alone.
- macOS: **Mini, Small and Medium controls keep rounded rectangles**. **Large and the new X-Large become capsules**. Capsules are "best used for standout actions" in dense desktop layouts. **[Apple, 356; https://wwdcnotes.com/documentation/wwdc25-310-build-an-appkit-app-with-the-new-design/]**
- Window corners in macOS 26: about **16 pt for titlebar-only windows and 26 pt for windows with toolbars**, chosen to wrap the glass toolbar concentrically. **[3p: https://github.com/mansarip/mac-window-corners, https://lapcatsoftware.com/articles/2026/3/4.html]**

### 1.8 Measured macOS 26 control sizes (points) [3p: https://applecn.vercel.app/foundations/layout, read from AppKit 2026-09-06]
- **Button heights, mini / small / regular / large / xl:** 16 / 20 / 24 / 28 / 36. Shapes: r 4 / 5 / 6 / capsule / capsule. Label size 13 for regular and above.
- **Hit target:** 28 default, 20 minimum. **Layout margin:** 10. **Padding around bezelled controls:** ~12.
- **Common controls:**
  - Switch 54×24, knob 31×20, 2 inset.
  - Checkbox 16, r 4.
  - Slider track 4, thumb 20×16.
  - Segmented control h 24, r 6.
  - Text field h 24, r 6.
  - Search field h 24, capsule.
- **Lists and sidebar:** list row min 28, r 10. Sidebar w 240, rows 28, r 6.
- **Bars:** toolbar 52 with 28 items. Title bar 32.
- **Overlays:**
  - Menu w 200, items 24, r 12, item r 5.
  - Popover r 12.
  - Sheet r 16, scrim 0.45.
  - Alert w 260, r 16.
  - Dialog r 16.
- **Window:** r 16. Card r 10.
- Menu bar: 24 pt tall and fully transparent in Tahoe. **[3p: https://saglitz.com/design/macos-app-design]**

### 1.9 Motion
- Visuals and motion were "designed as one." The material reacts by **"instantly flexing and energizing with light"**, with a **gel-like flexibility**. **[Apple, 219]**
- **Morphing:** when you move between states, glass reshapes between each context's controls, keeping "a singular floating plane." Menus **pop open from the button** that opened them. Action sheets now **spring from their source** instead of the bottom of the screen. **[Apple, 219 + 356]**
- When a sheet is dragged up into deeper focus, the glass recedes: more opaque and slightly larger. Modal tasks pair glass with a dimming layer; parallel tasks don't. **[Apple, 356]**

### 1.10 Scroll edge effects
- **Soft** (default) and **hard** styles exist. **[Apple, 219 + 356]**
  - **Soft** dissolves content into the background under floating glass. Over dark content it switches to a light dim instead.
  - **Hard** is uniform and more opaque. It's mostly used on macOS for pinned headers, text controls and controls without backgrounds.
- **Rules [Apple, 356]:**
  - Use one effect per view, and don't stack soft with hard.
  - In split views, each pane gets its own effect at a matching height.
  - Edge effects "are not decorative": don't use them where nothing floats above the content.
- Sidebars are inset glass, and content (including scroll views and hero images) extends underneath them via *background extension effects*. **[Apple, 356]**

### 1.11 Accessibility [Apple, 219 + Meet with Apple 201]
- **Reduce Transparency**: glass gets frostier and hides more of what's behind.
- **Increase Contrast**: elements turn mostly black or white, with a **contrasting border**.
- **Reduce Motion**: weaker effects and **no elastic behaviour**.
- System colours were retuned for Light, Dark and Increased Contrast. Type is bolder and left-aligned in alerts and onboarding. **[Apple, 356]**

### 1.12 The correction (important for "calm")
Tahoe's look drew wide criticism: heavy corners, floating sidebars, and resize hotspots that the large radius pushed outside the window (https://www.macobserver.com/news/macos-tahoes-rounded-corners-are-breaking-basic-window-resizing). At WWDC 2026, macOS 27 reportedly toned it down **[3p, blogs, not verified against Apple's own pages]**:
- one shared window radius, about **16 pt** (https://notes.bremer-media.com/macos/corner-radius-sizes.html)
- sidebars back to a simple vertical divider instead of a floating inset
- a user slider from "ultraclear" to "fully tinted" glass
- menu glyphs off by default (http://stevebarnes.org/blog/1781076981/)

**Takeaway for Cove:** follow the restrained version, not the Tahoe launch version.

---

## 2. Well-crafted glass implementations: frames worth borrowing

Reference images are saved in `/workspace/projects/52e7094c-440c-4f43-9b7f-f1b831d9e8a2/(internal reference images, not committed: copyrighted)`, for internal reference only. Do not ship any of them.

| File | Source | What to study |
|---|---|---|
| `01-apple-liquid-glass-hero.jpg` | https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/ | Overall material language across devices |
| `02-apple-macos-tahoe-hero.jpg` | https://www.apple.com/newsroom/2025/06/macos-tahoe-26-makes-the-mac-more-capable-productive-and-intelligent-than-ever/ | Desktop layout: transparent menu bar, floating glass |
| `03-apple-tahoe-control-center.jpg` | Apple press image, via https://appleosophy.com/2025/09/15/macos-tahoe-released-with-liquid-glass-design-phone-app-apple-games-app-live-activities-and-more/ | Module grid (detail below) |
| `04-apple-music-dynamic-tab-bars.jpg` | Apple Newsroom (same page as 01) | Tab bar shrinking and morphing over content |
| `05-lumabar-glass-dock-windows.webp` | https://lumabar.xyz | Glass dock on a real Windows desktop |
| `06-lumabar-real-refraction.webp` | https://lumabar.xyz | Visible edge refraction over bright 3D art |
| `07-lumabar-glass-light.webp` | https://lumabar.xyz | Glass on a light wallpaper (legibility test) |
| `08-ms-acrylic-recipe.png` | https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic | Microsoft's material stack (blur, exclusion blend, tint, noise) |
| `09-ms-mica-alt-tabs.png` | https://learn.microsoft.com/en-us/windows/apps/design/style/mica | Mica Alt behind a tabbed title bar |
| `10-kube-liquid-glass-og.png` | https://kube.io/blog/liquid-glass-css-svg | Rebuilt refraction plus rim highlight |

**Specific visual details to borrow (from the images):**

- **Control Center (03):**
  - Modules are capsules and circles on a loose grid with roughly even ~12 px gutters.
  - Each module has a hair-thin bright rim, brighter on the top-left and bottom-right edges (two-crescent specular).
  - The wallpaper's blue tints every module. That's adaptive tint doing the colour work, not paint.
  - **The "on" state is a solid white circle with a coloured glyph**, while "off" is plain glass. One strong state cue, no glows.
  - Sliders: a white fill on a translucent track inside a large rounded module. Labels are white, semibold, small.
  - The "Edit Controls" button is a small outlined capsule, i.e. a *quiet* secondary action.
- **lumabar dock (05/06/07):** the closest Windows precedent.
  - A long pill with a 1 px light rim.
  - Widgets sit inside it as **nested rounded cards whose radius = dock radius − padding** (concentric), each a slightly darker fill rather than more glass. That's correct no-glass-on-glass.
  - Accent colour appears only in the clock tile and the play button.
  - Refraction shows only at the outer edge, where the wallpaper bends.
  - Per user reviews on the site, it idles ("relaxation mode") when nothing behind it changes, which is a good battery pattern.
- **Apple Music tab bar (04):** glass bar that shrinks on scroll. A useful reference for a compact Cove toolbar that collapses when you scroll.

---

## 3. Translating to Windows

### 3.1 What Windows 11 gives you for free
| Feature | What it is | API | Cost / notes |
|---|---|---|---|
| **Mica** | Opaque, wallpaper-tinted backdrop for long-lived windows | WinUI 3 `MicaBackdrop` / `Window.SystemBackdrop`; Win32 `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE = 38, DWMSBT_MAINWINDOW)` | **Cheap: "only samples the desktop wallpaper once."** Turns neutral automatically when the window is inactive. **[MS]** |
| **Mica Alt** | Stronger tint, made for tabbed title bars | `MicaKind.BaseAlt`; DWM `DWMSBT_TABBEDWINDOW` | Windows App SDK 1.1+, build 22000+ **[MS]** |
| **Acrylic** | Frosted glass: blur, exclusion blend, tint, noise | `DesktopAcrylicBackdrop` / `AcrylicBrush`; DWM `DWMSBT_TRANSIENTWINDOW` | **GPU-intensive and shortens battery life.** Disabled in Battery Saver. Meant for transient surfaces (menus, flyouts). Don't place two panes side by side (visible seam) or layer them. **[MS]** |
| **Smoke** | Black dimming layer behind modals | Built into ContentDialog | Matches Apple's modal dimming **[MS]** |
| **Rounded corners** | DWM-drawn corners | `DWMWA_WINDOW_CORNER_PREFERENCE = 33` → `DWMWCP_ROUND` (8 px) / `DWMWCP_ROUNDSMALL` (4 px) / `DONOTROUND` | Build 22000+. **DWM's corners can't be larger than 8 px**, so a 12–16 px window needs a frameless/transparent window with the shape drawn in-app. **[MS enum page; the 8/4 px values are from https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners, not re-fetched]** |
| Border / caption colour | Hairline window border | `DWMWA_BORDER_COLOR` (`DWMWA_COLOR_NONE` hides it) | Build 22000+ **[MS]** |
| Dark frame | | `DWMWA_USE_IMMERSIVE_DARK_MODE = 20` | **[MS]** |
| Host backdrop for custom Composition | | `DWMWA_USE_HOSTBACKDROPBRUSH` + `Compositor.CreateHostBackdropBrush` | Build 22000+ **[MS]** |

DWM enum reference: https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute. DWM_SYSTEMBACKDROP_TYPE values: AUTO 0, NONE 1, MAINWINDOW 2 (Mica), TRANSIENTWINDOW 3 (Acrylic), TABBEDWINDOW 4 (Mica Alt). The values are from memory of the linked enum page; check before shipping.

**Fallbacks are automatic.** Mica and Acrylic become solid colours when:
- Transparency effects is off
- Battery Saver is on
- the hardware is low-end
- the window is inactive (Mica)
- the build is older than 22000

In High Contrast, the user's chosen colours replace them. **[MS]** This lines up with Apple's Reduce Transparency, so honour it the same way.

**Mapping Microsoft's layers onto Apple's layers [inferred]:**
- Mica base ≈ Apple's window background.
- WinUI `LayerFillColorDefaultBrush` content layer ≈ Apple's content layer.
- Acrylic flyouts ≈ Apple's menus and popovers.

What Windows lacks natively: **lensing/refraction, rim highlights, interaction glow, morphing, and adaptive light/dark switching.** Those need custom rendering.

### 3.2 What needs custom rendering
**Option A: native, WinUI 3 / Windows App SDK + Composition**
- `Microsoft.UI.Composition` effect brushes (Win2D `GaussianBlurEffect`, `CompositeEffect`, `ArithmeticComposite`). Direct2D also has a built-in **Displacement Map effect** (`CLSID_D2D1DisplacementMap`), so bezel refraction of the **app's own content** is doable natively.
- **Limit [inferred, verify]:** an app can't read the live pixels of other windows behind it for a custom shader. Only DWM's system backdrops see them.
  - Refracting *in-app* content (Cove's own scroll views under a glass toolbar) is fine.
  - Refracting the *desktop* means using the wallpaper image (as Mica does) or screen capture via `Windows.Graphics.Capture`. Capture is costly and raises privacy concerns.
  - lumabar says its glass "bends your wallpaper live." How it does that isn't documented.

**Option B: Tauri (WebView2 = Chromium)**
- Transparent window plus Mica or Acrylic from `window-vibrancy` (`apply_mica`, `apply_acrylic`) as the base layer. Set `html, body {background: transparent}` and `"transparent": true`. https://github.com/tauri-apps/window-vibrancy
- The crate's own warning: `apply_acrylic` and `apply_blur` have **"bad performance when resizing/dragging"** on several Windows builds. **`apply_mica` has no such warning, so prefer Mica.**
- Inside the webview, Chromium supports **SVG filters in `backdrop-filter: url(#lens)`**. That's the kube.io technique, and it works on in-page content. Safari and Firefox lack it, which doesn't matter in WebView2.
- WebGL is the fallback for full shader control (https://github.com/archisvaze/liquid-glass).

### 3.3 GPU and battery notes
- **Cost ranking, cheapest first [MS + inferred]:**
  1. Solid fill
  2. Mica (one wallpaper sample)
  3. Static blur of a known image
  4. Live Acrylic or `backdrop-filter: blur()` (re-composited whenever what's behind changes)
  5. SVG displacement or WebGL refraction (extra full-res passes per frame)
  6. Desktop capture plus a shader
- **Keep it cheap:**
  - Keep refraction on **small, few surfaces** (toolbar, dock, a single floating control).
  - Blur radius drives cost, so stay moderate (≤ 24 px).
  - Animate `scale` rather than rebuilding displacement maps. Rebuilds happen on every resize, so cache maps per size bucket.
  - **Pause or flatten when nothing behind changes**, when the window is inactive, in Battery Saver, or when transparency is off.
- Respect `UISettings.AdvancedEffectsEnabled` (the Windows Transparency effects switch) and battery state. In web code also check `prefers-reduced-transparency`, `prefers-contrast` and `forced-colors`. Whether WebView2 maps the Windows transparency setting to `prefers-reduced-transparency` is **unverified**, so pass it from Rust as a class on `<html>`.

---

## 4. Proposed "Cove Glass" design spec

**Principle:** Cove Glass is the restrained version of the idea, closer to macOS 27's correction than to Tahoe at launch.
- Glass is **only** for the floating control layer.
- Content sits on calm, near-solid surfaces.
- Light does the work, not colour.

### 4.1 Layers
| Layer | Windows backing | Cove token |
|---|---|---|
| L0 Window base | Mica (Tauri `apply_mica` / WinUI `MicaBackdrop`) | `surface.base`: transparent so Mica shows; solid fallback `#F5F5F7` light / `#1C1C1E` dark *(Cove's own neutrals; tune later)* |
| L1 Content | In-app solid layer | `surface.content`: light `rgba(255,255,255,0.72)` / dark `rgba(28,28,30,0.72)` |
| L2 Cove Glass (toolbar, sidebar header, dock, floating controls, menus) | Custom: blur + tint + rim + optional bezel refraction | see 4.3 |
| Scrim | Smoke equivalent | `scrim`: `rgba(0,0,0,0.40)` modal; `0.35` behind clear glass over bright media (HIG value) |

### 4.2 Radii (concentric system)
| Token | Value | Use |
|---|---|---|
| `radius.window` | **12 px** (drawn in-app; DWM ROUND = 8 px is the fallback for native frames) | Window. Sits between Win11's 8 and macOS 27's ~16 so it feels soft but not "Tahoe" |
| `radius.panel` | 12 | Sheets, dialogs, popovers |
| `radius.menu` | 10; items 6 | Menus |
| `radius.card` | 10 | Content cards |
| `radius.control.sm / md` | 5 / 6 | Compact buttons, fields, segmented controls (rounded-rect) |
| `radius.control.lg` | capsule (h/2) | 32–36 px primary/standout buttons, search, dock |
| Rule | `inner = outer − padding`, min 4. Use "concentric with fallback": a component standalone uses its fixed radius; nested, it computes. | |

### 4.3 Glass material tokens
| Token | Regular (default) | Clear (media only) |
|---|---|---|
| `glass.blur` | 20 px (dock/toolbar), 28 px (menus/popovers: "thicker when bigger") | 8 px |
| `glass.saturate` | 1.4 | 1.2 |
| `glass.tint.light` | `rgba(255,255,255,0.55)` | `rgba(255,255,255,0.12)` |
| `glass.tint.dark` | `rgba(30,30,32,0.55)` | `rgba(0,0,0,0.12)` + required 35% dim behind if content is bright |
| `glass.stroke` (outer hairline) | 1 px `rgba(255,255,255,0.35)` light / `rgba(255,255,255,0.12)` dark | 1 px `rgba(255,255,255,0.25)` |
| `glass.rim` (specular) | inset 1 px, two crescents: light direction −60° (top-left), peak `rgba(255,255,255,0.50)`, opposite edge 0.20, fading to 0 within 30% of the perimeter | same |
| `glass.inner-shade` | inset 0 −1px 0 `rgba(0,0,0,0.06)` | — |
| `glass.refraction` | **Off by default.** Optional "Lens" setting: convex-squircle bezel 12 px, IOR 1.5, scale ≤ 12 px, dispersion 0 | off |
| `glass.noise` | 2% monochrome grain (helps avoid banding, as Acrylic does) | 0 |
| Shadow `elev.1` (floating control) | `0 1px 2px rgba(0,0,0,0.08), 0 4px 12px rgba(0,0,0,0.10)` | |
| Shadow `elev.2` (menu/popover) | `0 2px 6px rgba(0,0,0,0.10), 0 12px 32px rgba(0,0,0,0.16)` | |
| Adaptive shadow | Raise shadow alpha +0.06 when the region behind is high-contrast or text-heavy (sample luminance variance a few times a second, not every frame) | |
| Inactive window | tint +0.15 opacity, rim −50%, refraction off ("recedes") | |

**Adaptive light/dark:** small glass (≤ 48 px tall: dock, toolbar pills) may switch to dark glass when the average brightness behind it is below 0.35, using ~0.1 hysteresis and a 200 ms crossfade. Large glass (menus, sidebar) never switches. **[from Apple, 219]**

**Tint/accent:** use one accent, `accent` = a calm blue or teal (Cove's own; *not* Apple system blue). Accent-tinted glass only for the single primary action in a view. Render tint as a 30–45% colour overlay over the glass, never as an opaque fill.

### 4.4 Spacing and sizes (desktop, 4 px grid)
- **Spacing scale:** 2, 4, 8, 12, 16, 20, 24, 32, 40.
- **Window and padding:**
  - Window margin 16.
  - Padding inside glass bars 6–8, so control radius = bar radius − padding.
  - Gutters between grouped glass controls 8. Gap between separate groups 12.
- **Control heights:**
  - compact 24 (r 6)
  - regular 28 (r 6)
  - large 32 (capsule)
  - x-large 36 (capsule; at most one per view)
  - Minimum hit target 28 px, and 32 when the target is mouse-only and isolated.
- **Bars and rows:**
  - Toolbar 48 tall (items 28).
  - Title-bar strip 32.
  - Sidebar 240 wide, rows 28, selection r 6.
  - List rows ≥ 32.
- **Specific controls:**
  - Switch 40×22 (knob 18, inset 2). Cove's own proportions, deliberately not Apple's 54×24.
  - Slider track 4, thumb 16.
- **Window controls:** use Windows-standard caption buttons on the right (min / max / close). **Don't** use traffic-light dots. They're an Apple signature, and Windows users expect the controls on the right.

### 4.5 Type (Inter, free under the OFL)
| Token | Size / line | Weight | Tracking |
|---|---|---|---|
| `display` | 28 / 34 | 600 | −0.02em |
| `title` | 20 / 26 | 600 | −0.01em |
| `headline` | 15 / 20 | 600 | −0.005em |
| `body` | 13 / 18 | 400 | 0 |
| `callout` | 12 / 16 | 500 | 0 |
| `caption` | 11 / 14 | 500 | +0.01em |
- Enable Inter features `cv11`, `ss01` and tabular numbers (`tnum`) for figures.
- Text on glass is ≥ 500 weight at ≤ 13 px. Glass text is always the primary label colour, not accent (MS: accent text on acrylic fails contrast at 14 px).
- Left-align alerts (matches Apple 356).
- Label colours:
  - Light: `rgba(0,0,0,0.88)` / `0.60` / `0.38`
  - Dark: `rgba(255,255,255,0.92)` / `0.62` / `0.40`

### 4.6 Motion
- **Default spring:** stiffness 300, damping 30 (about 250 ms with no visible bounce). Morphing and position changes use this.
- **Hover:** rim brightens +0.10 over 120 ms ease-out.
- **Press:** "illuminates from within." A radial glow from the pointer, white at 12% opacity, radius 1.5× the element height, rising over 90 ms and fading over 240 ms. Scale 0.98.
- **Menus and popovers grow out of the control that opened them:** transform-origin is the trigger. Scale 0.96→1 plus refraction/blur ramp over 180 ms. **Don't** cross-fade opacity alone.
- **Toolbar/dock collapse on scroll:** height and width morph over 220 ms, at most one morph at a time.
- **Reduce Motion** (Windows "Animation effects" off / `prefers-reduced-motion`):
  - no springs: 120 ms linear fades
  - no glow travel
  - no elastic overshoot

### 4.7 Scroll edge
- **Soft (default)** under floating glass: a 24 px progressive mask, plus a blur ramp of 0→8 px, on the content at the top edge. Over dark content, use a 0→0.25 black dim instead.
- **Hard** for pinned table headers: uniform `surface.content` at 0.92 opacity behind the toolbar and header together.
- One effect per pane, at matching heights, and only where something actually floats.

### 4.8 Accessibility modes
| Setting (Windows source) | Cove behaviour |
|---|---|
| Transparency effects off (`UISettings.AdvancedEffectsEnabled = false`) / Battery Saver | Glass → frosted solid: tint opacity 0.92, no blur, no refraction, rim kept as 1 px stroke |
| High Contrast / `forced-colors` | System colours only, 2 px `CanvasText` borders, no tint, no shadow, no glass |
| `prefers-contrast: more` (Increase Contrast equivalent) | Tint 0.85, mostly black or white glass, 1.5 px contrasting border, secondary labels at ≥ 0.75 alpha |
| Animation effects off | As 4.7 / 4.6 reduced |
| Text scaling | All sizes in rem/DIPs, so layout survives 200% |
- **Minimum contrast:** 4.5:1 for body, 3:1 for ≥ 15 px semibold and glyphs. Test against the **worst-case backdrop**: a white wallpaper, a busy photo, and black.

### 4.9 Do's
- Use glass only for the floating control layer: toolbar, dock, floating controls, menus/popovers. Content goes on calm surfaces.
- Group related controls into one glass container. Separate groups with space, not dividers.
- Use one primary tinted action per view at most.
- Make radii concentric, and check nested corners for pinch or flare.
- Let the wallpaper or content supply the colour. Keep Cove's chrome neutral.
- Grow menus out of their trigger, and let glass "materialize" through blur and lens ramps.
- Default to Mica plus light glass. Make refraction an opt-in "Lens" setting, and pause it when idle, inactive or on battery.
- Test every surface over white, black and a busy photo, in light and dark.

### 4.10 Don'ts
- No glass on glass. No glass list rows or cards.
- No mixing Regular and Clear in one view.
- No chromatic dispersion or rainbow fringes, no motion-reactive highlights, no animated wallpaper shimmer. That's the "flashy" end.
- No opaque accent fills on glass, and no accent-coloured text on glass.
- No glass sitting over content in the app's resting state.
- No large radii that eat resize hotspots. Keep the window radius ≤ 12 px, and give an 8 px resize zone that extends *outside* the visible corner (the Tahoe lesson).
- Don't stack scroll edge effects. Don't use them as decoration.
- No Apple assets: no SF Symbols, no SF Pro, no Apple wallpapers, no traffic-light window controls, no "Liquid Glass" naming in the product. Use an open icon set (for example Lucide or Phosphor) and Inter.

---

## 5. Open questions / to verify
1. Check the DWM corner pixel values and DWM_SYSTEMBACKDROP_TYPE numbers against the live docs. (The enum page above confirms the attribute names; the numeric backdrop values and 8/4 px come from the linked pages and memory.)
2. Does WebView2 expose `prefers-reduced-transparency` from the Windows setting? Until confirmed, pass it from native code.
3. Is desktop-wallpaper refraction worth it? Cheapest route: read the wallpaper path from the registry plus `IDesktopWallpaper`, blur a copy once, and use it as the refraction source (like Mica). Avoid live screen capture.
4. Check the macOS 27 claims (single ~16 pt radius, glass slider) against Apple's WWDC26 sessions or HIG change log before citing them externally.
5. The applecn control measurements are third-party. They're fine as proportions, not as specs to copy.
