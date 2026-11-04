# Cove Glass & Apple-grade UI: the practical guide

*Written 2026-11-03 for Cove (Tauri 2, WebView2 + Rust, Windows). It distils Apple's current HIG, the best open-source Liquid Glass work and real WebView2 behaviour into rules with numbers. It sits on top of the longer research note `docs/research/05-liquid-glass-design.md` (the "Cove Glass" spec) and the Cove rules in `docs/design/REDESIGN.md`. Where they disagree, REDESIGN.md wins (no gradients, glow blobs or purple; Ink #111215; teal #2cc9b9).*

Labels: **[Apple]** = Apple HIG / WWDC. **[MS]** = Microsoft. **[3p]** = third-party source, cited. **[tested]** = rendered in headless Chromium for this guide. **[Cove]** = our own call, a starting value to tune by eye.

---

## 0. The one fact that decides everything

**A WebView2 page cannot see the desktop behind its window.** CSS `backdrop-filter` (blur or SVG) only filters pixels *inside the page*. In a transparent Tauri window it has nothing to sample, so `backdrop-filter: blur(24px)` over the wallpaper does **nothing**: you see the tint and nothing else.
- Tauri docs recipe (JP): "`backdrop-filter: blur()` only blurs elements in the page, not the desktop behind the window." https://tauri.ninja/recipe.php?id=win-008
- WebView2 team: "CSS's backdrop-filter only applies to elements within the webpage, and the content beneath the browser window is not included." https://github.com/MicrosoftEdge/WebView2Feedback/issues/4945
- Two transparent elements with `backdrop-filter` over a transparent body don't even blur each other (WebView2 148). https://github.com/tauri-apps/tauri/issues/15512
- Another app removed its Windows "blurry" mode for this reason. https://github.com/KurtGokhan/KeyParty/commit/939b3a4fca2510748f4df98086be204dc2c6f5aa
- The Na2H2P2O7 liquid-glass skill is built on the same insight: the host feeds the wallpaper (or a capture) to the renderer as a texture. https://github.com/Na2H2P2O7/liquid-glass-skill

**What this means for Cove today:** `ui/src/dock.css` puts `backdrop-filter: blur(24px) saturate(1.4)` on the dock pill in a transparent window, so the dock shows only `--pill` tint. That blur costs GPU time and draws nothing. (Observation only; I did not change the file.)

**So Cove has three ways to get real glass, chosen per surface:**

| Surface | What is behind it | Technique |
|---|---|---|
| **Dock, menu bar** (they reserve screen space, so app windows never go under them) | Always the wallpaper | **Wallpaper plate**: an in-page slice of the actual wallpaper, lined up to screen coordinates, frosted and refracted by an SVG filter. This is the only way to get real lensing. **Recommended.** |
| **Control Centre, menus, popovers, Search** (float over app windows) | Anything | **DWM backdrop** via `window-vibrancy` (`apply_acrylic` / `apply_mica`; already used in `main.rs`), with CSS tint and rim on top. Real blur of live content, but **no refraction**. Or the Thick tier (§3) over a wallpaper plate if the panel only ever covers the desktop. |
| **Overlays inside a Cove window** (sheets in Settings, menus over Settings content) | Opaque page content | Plain CSS `backdrop-filter: blur() url(#f)`. Works because the page under it is opaque. Chromium only, which is fine in WebView2. |

`window-vibrancy` caveats on Windows: `apply_acrylic` has "bad performance when resizing/dragging the window on Windows 10 v1903+ and Windows 11 build 22000"; `apply_blur` the same on 22621+; `apply_mica` has no such note. https://github.com/tauri-apps/window-vibrancy. Acrylic is "GPU-intensive", is turned off in Battery Saver, and is turned off by the user's *Transparency effects* switch [MS] https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic. Static windows like the Control Centre don't hit the drag cost.

---

## 1. Apple's rules that matter for Cove (from the HIG pages, read 2026-11-03)

**Materials / Liquid Glass** [Apple] https://developer.apple.com/design/human-interface-guidelines/materials
1. Liquid Glass is a **functional layer for controls and navigation that floats above content**. Never use it in the content layer. The one exception: sliders and toggles take on glass *while being used*.
2. **Use it sparingly**: "limit these effects to the most important functional elements." For Cove that means the dock, the menu bar, Control Centre and transient menus. Not Settings cards and not list rows.
3. **Regular** variant (blurs, adjusts luminosity) is for anything with text: alerts, sidebars, popovers. **Clear** variant only over visually rich media. If the content behind clear glass is bright, add a **35% dark dimming layer**.
4. Content-layer structure uses **standard materials** (blur and vibrancy). Pick a material by meaning, not by the colour it gives, because system settings change its look. Use vibrant (adaptive) label colours on materials. Thicker = more opaque = better for fine text.

**Menu bar** [Apple] https://developer.apple.com/design/human-interface-guidelines/the-menu-bar
- **Height 24 pt.** Extras sit at the trailing end, and app menus win space when crowded (extras get hidden first).
- Menu-bar icons are **single-colour template glyphs** (black and clear), so the system can recolour them for light and dark bars and the selected state.
- **Clicking an extra opens a menu, not a popover**, unless the feature is too complex for a menu. (Control Centre is the accepted exception.)
- **Short, one-word titles**, in title case when longer.
- The user, not the app, decides which extras live in the bar. Don't rely on an extra being visible.

**Dock** [Apple] https://developer.apple.com/design/human-interface-guidelines/dock-menus. There is no HIG page for the Dock surface itself, only Dock menus. Keep items short and logical. List open windows plus a few high-value actions. Every Dock-menu command must also exist elsewhere.

**Motion** [Apple] https://developer.apple.com/design/human-interface-guidelines/motion
- Motion must be purposeful and **brief and precise**. **Avoid motion on interactions that happen often.** **Let people cancel motion**: never block input until an animation ends. Motion must be optional and never the only signal.
- Liquid Glass motion is "more subdued" with a trackpad or mouse than with touch. Cove is mouse-first, so stay calm.

**Settings** [Apple] https://developer.apple.com/design/human-interface-guidelines/settings
- Good defaults, **few settings**, and **don't duplicate system settings** (accessibility, scrolling): honour them instead. Put task options where the task is.
- **Ctrl+, opens Settings** (Cmd+, on Mac). **Reopen the last-viewed pane.** **Window title = current pane.** Keep a stable, non-customisable navigation that always shows the active pane. A settings window needs no maximise.

**WWDC25 material behaviour** [Apple] (from 05-research §1, sessions 219/356): lensing is "the primary way Liquid Glass visually defines itself". Glass **appears by turning lensing up, not by fading opacity**. **Bigger glass = thicker** (more shadow, stronger lensing, softer scatter). **Small elements flip light/dark** with the backdrop and **large ones don't**. **Never glass on glass.** Concentric radii: child radius = parent radius − padding.

**The backlash, and why Cove stays calm** [3p] NN/g, "Liquid Glass Is Cracked": transparency over busy backgrounds hurts legibility, controls "insist on animating themselves", and touch targets shrank. https://www.nngroup.com/articles/liquid-glass. Apple itself added a Clear/Tinted switch in iOS 26.1 (Armitanemati skill §1). Default to the **more opaque, more legible** side.

---

## 2. Values: the Cove Glass token set

All [Cove] starting values, derived from the sources named in each row. Tune on the four Cove wallpapers in **both** light and dark, and on the user's own (busiest) wallpaper.

### 2.1 Blur, saturation, brightness

| Tier | Frost (blur of the plate or DWM) | saturate | brightness | Source / note |
|---|---|---|---|---|
| Clear (optional dock style) | 0–2 px | 1.5 | 1.0 | refraction does the work; xinlingfeiwu "refractive blur ~0.3px" |
| **Regular** (dock, menu bar backing) | **20–24 px** | **1.6–1.8** | 0.95 dark / 1.05 light | Armitanemati Regular 20–28 px; Josh Comeau `blur(8–16px) saturate(140%)` |
| **Thick** (menus, Control Centre, Search) | 28–32 px (or DWM acrylic) | 1.4–1.6 | 0.9 dark | Armitanemati Thick 24–32 px |
| Chips / small repeated items | 6 px, no refraction | 1.25 | 1.0 | stormaref "chip" tier |

- Saturation above **1.8 reads as candy**. Armitanemati puts the ceiling at 180%. Keep brightness and contrast tweaks within **±10%**.
- **Never animate blur radius** on big surfaces. Animate opacity, transform or the displacement `scale`.
- Over a flat or soft backdrop, blur is invisible and pure cost: max pixel difference of 3/255 in Armitanemati's measurement. Cove's wallpapers are soft, so the **rim, shadow and tint carry the material** more than the blur does.

### 2.2 Tint (fill) opacity, by contrast maths

Worst-case maths from Armitanemati: a dark fill (~#16161a) with white text needs **≥0.59 opacity** for 4.5:1 over pure white, and ≥0.46 for 3:1. A light fill (~#f5f5f7) with black text needs **≥0.51** over pure black. Blur helps but is no guarantee.

| Tier | Dark mode fill (Ink #111215) | Light mode fill (#f7f3ef, warm white) | Text allowed |
|---|---|---|---|
| Clear | 0.10–0.18 + 35% dim if wallpaper is bright [Apple] | 0.08–0.15 | icons, bold labels ≥ 15 px |
| **Regular** | **0.30–0.42** | **0.35–0.50** | 13 px semibold labels, icons |
| **Thick** | **0.60–0.72** | **0.62–0.78** | body text, menus |
| Solid (reduced transparency, high contrast) | 0.96–1.0 | 0.96–1.0 | anything |

- **The glass is colourless, and the colour comes from behind.** Never tint glass teal or with logo colours. Accent goes on *one* element (the active dot, an "on" toggle) [Apple; stormaref rule 2].
- The "on" state in Control Centre is a **solid light circle with a teal glyph**, and "off" is plain glass. One strong cue, no glow (05-research §2).
- **A child of glass paints no surface of its own**: no background, border or radius of its own except a subtle state fill (`rgba(255,255,255,.08)` hover / `.14` pressed in dark; `rgba(0,0,0,.05)` / `.09` in light) [stormaref rule 3; Armitanemati].

### 2.3 Edge highlights (rim) and stroke: no gradients needed

The rim is what makes glass read as a physical object. Build it from inset shadows (Cove bans gradient fills):

```css
/* dark mode */
--cg-rim:
  inset 0 1px 0 rgba(255,255,255,.22),      /* top key light, 1px */
  inset 0 0 0 1px rgba(255,255,255,.10),    /* hairline all round */
  inset 0 -1px 0 rgba(255,255,255,.04);     /* faint bottom return */
/* light mode */
--cg-rim:
  inset 0 1px 0 rgba(255,255,255,.60),
  inset 0 0 0 1px rgba(255,255,255,.30),
  inset 0 -1px 0 rgba(255,255,255,.12);
/* outer separation stroke for light wallpapers (light mode only) */
--cg-stroke: 0 0 0 0.5px rgba(0,0,0,.10);
```
Sources: stormaref (rim top .50–.55, hairline .10–.12, bottom .06), xinlingfeiwu (`inset 0 1px 0 rgba(255,255,255,.55)` + 1px border .38). Cove's dark values are lower because Ink glass on a dark wallpaper shows a bright rim as a "neon outline". Apple's look is **two thin crescents** (top-left and bottom-right) [Apple 219 via 05-research §1.3]. The refraction recipe adds those optically. No full-surface white sheen, ever (xinlingfeiwu rejection list).

### 2.4 Shadow recipes

Shadows lift glass off the wallpaper. Big glass = deeper shadow [Apple 219].

```css
--cg-shadow-chip:  0 1px 2px rgba(0,0,0,.16);
--cg-shadow-bar:   0 1px 0 rgba(0,0,0,.06);                                   /* menu bar: almost none */
--cg-shadow-dock:  0 10px 30px rgba(0,0,0,.22), 0 2px 6px rgba(0,0,0,.12);
--cg-shadow-menu:  0 12px 32px rgba(0,0,0,.28), 0 4px 10px rgba(0,0,0,.14);   /* menus, popovers */
--cg-shadow-panel: 0 24px 60px rgba(0,0,0,.32), 0 6px 16px rgba(0,0,0,.16);   /* Control Centre */
/* light mode: multiply alphas by ~0.6 */
```
(stormaref's menu shadow `0 12px 32px .14 / 0 4px 10px .07` in light and `0 16px 40px .55 / 0 4px 12px .35` in dark brackets these values.)
- **A transparent window clips its own shadow.** The dock and panel windows must be larger than the visible glass by at least the shadow reach (**≥ 32 px** for dock, **≥ 64 px** for Control Centre) [Cove, from how CSS paints inside the WebView].
- Fully transparent pixels of a Tauri window still take mouse input on Windows unless you make them click-through (`set_ignore_cursor_events` toggled from hit-testing, or a window region) [Cove, verify on device].

### 2.5 Shape and spacing (concentric)

- **Concentric rule:** inner radius = outer radius − padding [Apple 356]. Dock pill r 26 with 7 px padding → tile hover r 19. Control Centre panel r 22, padding 12 → modules r 10 (or capsules).
- macOS 26 measured sizes [3p, 05-research §1.8]: menu w 200, item h 24, menu r 12, item r 5; popover r 12; window r 16; switch 54×24; slider track 4 / thumb 20×16; hit target ≥ 28 (min 20). Layout margin 10, bezel padding ~12.
- **Cove spacing scale** stays 4/8/12/16/24/32 (`tokens.css`). Gutters in Control Centre are **12 px**. Menu insets are 6 px around items and 5 px item radius.
- **Dock [Cove]:** icon 48 px (`--size`), gap 6 px, pill padding 7/10/9 (current), r 26. Magnification at most ×1.5 over a cosine falloff of about 3 icons. Indicator dot 4 px; active = 14×4, r 3, teal.
- **Menu bar [Cove]:** 24–28 px tall (HIG 24 pt; ≥ 26 px if the click target feels small on Windows at 100%). Glyphs 16 px, 13 px text, 10–12 px between extras.

### 2.6 Typography (Windows-native, Apple-grade)

Cove uses **Segoe UI Variable**, which ships optical families. Use them as Apple uses SF Text vs Display:
- ≤ 11 px: `"Segoe UI Variable Small"`. 12–17 px: `"Segoe UI Variable Text"`. ≥ 20 px: `"Segoe UI Variable Display"`. (Inter is the concept-render stand-in only.)
- Scale [Cove, mapped from macOS]: menu bar / menus / body **13 px** regular (600 for the app name); secondary 11–12 px; Control Centre module titles 13 px semibold with 11 px secondary; settings pane title 20 px semibold; page hero 28 px (`--text-title`).
- **Tracking by size, not fixed:** −0.02em at ≥ 20 px, ~0 for body, +0.01em at ≤ 11 px (emilkowalski apple-design quick reference).
- Weight carries hierarchy, not colour. **Text on glass is ≥ 500 weight under 14 px.** Use `-webkit-font-smoothing: antialiased`. Never put text on the refraction bezel band; keep it in the flat centre.
- Glyphs and text on glass flip with the glass: **small surfaces (menu bar) pick light or dark ink from the wallpaper's luminance**, and large ones (Control Centre) keep one appearance [Apple 219]. Cove knows the wallpaper, so sample the luminance under the bar **once per wallpaper or monitor change**, never per frame.

### 2.7 Motion: springs

Apple thinks in **damping ratio + response** (emilkowalski/skills `apple-design`):

| Use | Damping | Response | Physics (mass 1) | Settles |
|---|---|---|---|---|
| **Default UI** (menus open, panel move, hover lift) | **1.0** | **0.30–0.40 s** | stiffness 322, damping 35.9 @0.35 | ~514 ms |
| Small, fast feedback (press, toggle) | 1.0 | 0.25 s | stiffness 632, damping 50.3 | ~367 ms |
| **Only after momentum** (flicked dock item, thrown window) | **0.8** | 0.30 s | stiffness 439, damping 33.5 | ~404 ms, 1.5% overshoot |
| Drawer / sheet [Apple] | 0.8 | 0.30 | | |

CSS `linear()` versions (computed for this guide; set `transition-duration` to the settle time):
```css
--spring-ui:   linear(0, .057, .18, .32, .455, .573, .671, .75, .812, .86, .896, .924, .944, .96, .971, .979, .985, .989, .992, .994, .996, .997, .998, .999, 1); /* 1.0 / 0.35 → 514ms */
--spring-flick: linear(0, .051, .17, .315, .462, .597, .711, .803, .874, .926, .963, .987, 1.002, 1.011, 1.014, 1.015, 1.014, 1.012, 1.01, 1.008, 1.006, 1.004, 1.003, 1.002, 1); /* 0.8 / 0.30 → 404ms */
```
- **Respond on pointer-down** (scale 0.97, about 100 ms ease-out). Never wait for click-up. Dock magnification follows the pointer 1:1 with **no transition while hovering**, and springs back on leave.
- **Interruptible:** retarget from the live, on-screen value (read the computed transform) and carry velocity. A CSS transition retargets but drops velocity. Use a JS spring (e.g. Motion `type: 'spring', bounce: 0, duration: 0.4`) for anything draggable.
- **Enter and exit on the same path**: menus grow from the item that opened them, origin at the click point.
- Glass **appears by ramping the displacement `scale` 0 → value** (cheap, no map rebuild) plus opacity, not opacity alone [Apple 219; kube.io].
- Frequent interactions get almost no motion [Apple Motion].
- `prefers-reduced-motion`: crossfade at 150 ms, no scale and no spring. Keep the state feedback (`tokens.css` already zeroes durations).

---

## 3. Material tiers mapped to Cove surfaces

| Cove surface | Tier | Backdrop technique | Refraction |
|---|---|---|---|
| Dock pill | Regular (Clear as an opt-in look) | wallpaper plate, pre-frosted | yes, `strength 0.6–0.8`, bezel 16 px |
| Dock tooltip / label | Thick, or solid label (`--label-bg`) | none needed | no |
| Dock menu, stack popover | Thick | DWM acrylic window, or plate if over desktop only | no |
| Menu bar | transparent by default; Regular backing only when a window is maximised under it or the wallpaper is busy | plate | rim-only (bezel 6–8 px) or none |
| Control Centre | Thick, modules as fills inside (no nested glass) | DWM acrylic (covers apps) | no; sliders and toggles may flash glass while dragged |
| Search / launcher | Thick | DWM acrylic (current) | no |
| Settings window | **No glass**: Ink #111215 surface. Glass only on the live desktop miniature's dock and bar, and on sheets over content | in-page `backdrop-filter` | optional on the miniature |

---

## 4. Ready recipe: refraction glass for WebView2, with fallbacks

**Tested** in headless Chromium at DPR 1 and 1.5: edge lensing visible, flat centre undistorted, plate lines up with the desktop. Test harness: `/workspace/projects/52e7094c-440c-4f43-9b7f-f1b831d9e8a2/work/lgtest/` (`cove-glass.js`, `test.html`, `crop.png`). **Not yet run inside WebView2 on Windows. Do that first.**

### 4.1 How it works
1. Rust gives the page the **wallpaper image** for the window's monitor, the **monitor rect** and the **window's position** (CSS px). Cove already knows its scene files (`wallpaper.rs`), and `SPI_GETDESKWALLPAPER` gives the user's own. The wallpaper fit mode must match: Cove sets "Fill", so `background-size: cover` against the monitor rect.
2. Rust (or a one-off canvas) bakes **two versions**: sharp (≤ monitor res) and **pre-frosted** (Gaussian 20–24 px plus saturate 1.6, downscaled to ~720 px, since bilinear upscaling smooths for free per the Na2H2P2O7 pitfalls). Frost then costs **nothing at runtime**.
3. Each glass element holds a `.cg-plate` child showing the slice of that image that sits under it on screen.
4. A per-size **SVG `feDisplacementMap`** (convex squircle bezel, Snell n=1.5, identity in the centre) is applied to the plate with plain `filter:` (works in every Chromium, and avoids the backdrop-root and transparent-window issues). Convex glass samples only *inward*, so no bleed margin is needed (kube.io).
5. Tint, rim and shadow come from CSS layered above the plate. Content sits on top, sharp.

### 4.2 CSS

```css
.cg {                                   /* the glass surface */
  position: relative; overflow: hidden; isolation: isolate;
  border-radius: var(--radius-dock);    /* map reads the computed radius */
  box-shadow: var(--cg-rim), var(--cg-shadow-dock);
  background: rgba(17,18,21,.92);       /* solid baseline if anything fails */
}
.cg > .cg-plate {
  position: absolute; inset: 0; z-index: -2; pointer-events: none;
  background-image: var(--cg-wall-frost);              /* url(asset://…frost.jpg) */
  background-size: var(--cg-mon-w) var(--cg-mon-h);   /* monitor size in CSS px */
  background-position: var(--cg-x) var(--cg-y);       /* −(element.left + window.x − monitor.x) … */
  background-repeat: no-repeat;
  contain: strict;                                     /* keep icon animation from re-running the filter */
}
.cg::after {                            /* tint layer above the plate */
  content: ""; position: absolute; inset: 0; z-index: -1;
  border-radius: inherit; pointer-events: none;
  background: var(--cg-tint, rgba(17,18,21,.34));     /* Regular, dark */
}
html[data-mode="light"] .cg { --cg-tint: rgba(247,243,239,.42); }
.cg[data-tier="thick"] { --cg-tint: rgba(17,18,21,.66); }
.cg[data-tier="clear"] { --cg-tint: rgba(17,18,21,.14); }
.cg[data-tier="clear"] > .cg-plate { background-image: var(--cg-wall-sharp); }

/* Fallback 1: no plate yet or plate failed. The frosted DWM window or the solid baseline shows. */
.cg:not([data-plate]) > .cg-plate { display: none; }

/* Fallback 2: user turned off Windows "Transparency effects" (Chromium maps it, Chrome 118+) */
@media (prefers-reduced-transparency: reduce) {
  .cg > .cg-plate { display: none; }
  .cg::after { background: rgba(17,18,21,.97); }
  html[data-mode="light"] .cg::after { background: rgba(247,243,239,.97); }
}
/* Fallback 3: high contrast / forced colours */
@media (prefers-contrast: more) {
  .cg > .cg-plate { display: none; }
  .cg::after { background: Canvas; }
  .cg { box-shadow: inset 0 0 0 1px CanvasText; }
}
@media (forced-colors: active) { .cg > .cg-plate, .cg::after { display: none; } .cg { border: 1px solid CanvasText; } }
/* Fallback 4: Battery Saver / "reduce effects" Cove setting → frost without refraction */
html[data-effects="low"] .cg > .cg-plate { filter: none !important; }
/* Reduced motion */
@media (prefers-reduced-motion: reduce) { .cg, .cg * { transition-duration: 150ms !important; transition-property: opacity !important; } }
```

### 4.3 SVG filter generated per size (JS, drop-in)

```js
// cove-glass.js: refraction map for a rounded rect, cached per [w,h,r,opts]
const NS = 'http://www.w3.org/2000/svg', cache = new Map(); let defs;
const bank = () => (defs?.isConnected ? defs : (() => {
  const s = document.createElementNS(NS, 'svg'); s.setAttribute('width', 0); s.setAttribute('height', 0);
  s.style.position = 'absolute'; s.setAttribute('aria-hidden', 'true');
  defs = s.appendChild(document.createElementNS(NS, 'defs')); document.body.appendChild(s); return defs; })());

function profile(bezel, thick, ior, n = 128) {             // px shift across the bezel (kube.io)
  const f = x => Math.pow(1 - Math.pow(1 - x, 4), 0.25);   // convex squircle
  return Float32Array.from({ length: n }, (_, i) => {
    const x = Math.min(Math.max(i / (n - 1), 5e-4), 1 - 5e-4), e = 1e-3;
    const slope = thick * (f(Math.min(x + e, 1)) - f(Math.max(x - e, 0))) / (2 * e * bezel);
    const t1 = Math.atan(slope), t2 = Math.asin(Math.sin(t1) / ior);
    return thick * f(x) * Math.tan(t1 - t2);
  });
}

function buildMap(w, h, r, { bezel, thickness, ior }) {
  const mags = profile(bezel, thickness, ior), max = Math.max(...mags, 1e-3);
  const c = Object.assign(document.createElement('canvas'), { width: w, height: h });
  const ctx = c.getContext('2d'), img = ctx.createImageData(w, h), hw = w / 2, hh = h / 2;
  for (let py = 0; py < h; py++) for (let px = 0; px < w; px++) {
    const x = px + .5 - hw, y = py + .5 - hh, qx = Math.abs(x) - (hw - r), qy = Math.abs(y) - (hh - r);
    const ox = Math.max(qx, 0), oy = Math.max(qy, 0);
    const d = -(Math.hypot(ox, oy) + Math.min(Math.max(qx, qy), 0) - r);   // depth inside edge
    let dx = 0, dy = 0;
    if (d > 0 && d < bezel) {
      let nx, ny;
      if (qx > 0 && qy > 0) { const l = Math.hypot(ox, oy) || 1; nx = ox / l; ny = oy / l; }
      else if (qx > qy) { nx = 1; ny = 0; } else { nx = 0; ny = 1; }
      nx *= Math.sign(x) || 1; ny *= Math.sign(y) || 1;
      const m = mags[Math.floor(d / bezel * (mags.length - 1))] / max;
      dx = -nx * m; dy = -ny * m;                    // sample inward (convex keeps rays inside)
    }
    const i = (py * w + px) * 4;
    img.data[i] = Math.round(128 + dx * 127); img.data[i + 1] = Math.round(128 + dy * 127);
    img.data[i + 2] = 128; img.data[i + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  return { url: c.toDataURL(), scale: 2 * max };    // shift = scale·(C/255 − .5)
}

export function refractionFilter(w, h, r, o = {}) {
  w = Math.round(w); h = Math.round(h); r = Math.min(r, w / 2, h / 2);
  o = { bezel: Math.min(o.bezel ?? 16, w / 2, h / 2), thickness: o.thickness ?? 32, ior: o.ior ?? 1.5, strength: o.strength ?? .7 };
  const key = [w, h, r, o.bezel, o.thickness, o.ior, o.strength].join('|'), hit = cache.get(key);
  if (hit?.node.isConnected) return hit.id;
  const { url, scale } = buildMap(w, h, r, o), id = 'cg-' + cache.size;
  const f = document.createElementNS(NS, 'filter');
  for (const [k, v] of Object.entries({ id, x: 0, y: 0, width: w, height: h, filterUnits: 'userSpaceOnUse',
    primitiveUnits: 'userSpaceOnUse', 'color-interpolation-filters': 'sRGB' })) f.setAttribute(k, v);
  f.innerHTML = `<feImage href="${url}" x="0" y="0" width="${w}" height="${h}" preserveAspectRatio="none" result="map"/>
    <feDisplacementMap in="SourceGraphic" in2="map" scale="${(scale * o.strength).toFixed(2)}" xChannelSelector="R" yChannelSelector="G"/>`;
  bank().appendChild(f); cache.set(key, { id, node: f }); return id;
}

// win = { x, y } window origin and mon = { x, y, w, h } monitor rect, CSS px, sent from Rust
export function mountGlass(el, win, mon, opts) {
  const plate = el.querySelector('.cg-plate');
  const apply = () => {
    const b = el.getBoundingClientRect(); if (!b.width) return;
    const r = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
    plate.style.filter = `url(#${refractionFilter(b.width, b.height, r, opts)})`;
    el.style.setProperty('--cg-x', `${mon.x - win.x - b.left}px`);
    el.style.setProperty('--cg-y', `${mon.y - win.y - b.top}px`);
    el.style.setProperty('--cg-mon-w', `${mon.w}px`); el.style.setProperty('--cg-mon-h', `${mon.h}px`);
    el.dataset.plate = '';
  };
  const ro = new ResizeObserver(() => requestAnimationFrame(apply)); ro.observe(el); apply();
  return () => ro.disconnect();
}
```

Usage:
```html
<div class="cg dock" data-tier="regular"><div class="cg-plate"></div> …icons… </div>
<script type="module">
  import { mountGlass } from './cove-glass.js';
  mountGlass(document.querySelector('.dock'), win, mon, { bezel: 16, thickness: 32, strength: .7 });
</script>
```

**Tuning [tested + Cove]:** `bezel 22 / thickness 48 / strength 1` is clearly visible and slightly busy at the rim (good for the "Clear" look). **Cove default `bezel 16 / thickness 32 / strength 0.6–0.8`.** Menu bar backing: `bezel 6–8`. Keep the bezel under 1/3 of the element's short side, and keep text out of it.

**Rules that keep it correct** (stormaref refraction.md, xinlingfeiwu golden-glass-style.md):
- **Cache maps by `[w,h,r,opts]`** and share them: same-size dock tiles use one filter. Rebuild only on resize, never on pointer move. Dock magnification changes icon size, not the pill's filter. If the pill width animates, **swap the map at the end** or rebuild at most once per frame on a debounced ResizeObserver.
- **Check `node.isConnected`** before reusing: a detached `<defs>` makes `url(#id)` resolve to nothing, so the panel goes blank (and the solid baseline then shows).
- **Centre must be exactly identity.** Never use `feTurbulence` or a linear-gradient map, which wobble the whole pane like heat haze (Armitanemati §1.5).
- Chromatic dispersion, if ever: rim only, per-channel scale ±7%, combined with `feBlend screen`. Default **off** for Cove (calm), and never `feOffset` on whole channels.
- **Animate `scale` on the `feDisplacementMap`** to bring glass in and out, never the map itself.
- 8-bit maps cap the shift near ±127 px, far more than needed. The neutral-128 bias drift is `scale/510` px, which is negligible at these scales (stormaref).

### 4.4 In-page overlays (Settings sheets, menus over Settings content)
The page under them is opaque, so plain backdrop-filter works, SVG included (Chromium only, which WebView2 is):
```css
.sheet-glass {
  background: rgba(17,18,21,.62);
  -webkit-backdrop-filter: blur(24px) saturate(1.6);
  backdrop-filter: blur(24px) saturate(1.6);
  box-shadow: var(--cg-rim), var(--cg-shadow-menu);
}
/* refraction: put the url() in a separate rule so a parse failure only drops this rule */
.sheet-glass.refract { backdrop-filter: blur(2px) url(#cg-3) saturate(1.6); }
```
- `@supports` can't detect SVG-in-backdrop support (Safari parses it and paints nothing). In WebView2 it is always Chromium, so no gate is needed. Gate by engine if this code is ever shared (stormaref rule 4).
- **Backdrop roots:** `filter`, `opacity < 1`, `mask`, `clip-path`, `mix-blend-mode` or another `backdrop-filter` on an ancestor stop the backdrop there (MDN https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/backdrop-filter). So **never nest glass**, and never fade a glass panel's *parent* with opacity: fade the panel itself.
- Chrome renders **neither** effect on an element with both `mask-image` and `backdrop-filter` (Armitanemati). Put the mask on a wrapper.
- `overflow: hidden` clipping before blur was fixed in **Chrome 142** (Josh Comeau https://www.joshwcomeau.com/css/backdrop-filter/). WebView2 is evergreen, so it's fine.

---

## 5. WebView2 performance & platform notes

- **Cost model:** each `backdrop-filter` re-samples and blurs on every frame where something under it changes. Armitanemati measured headless (software) Chromium at 16.7 ms median with 0 filtered layers, 20 ms with 1, 29 ms with 5, and 79 ms with 9. GPUs do far better but the curve has the same shape. **Keep ≤ 2 live filtered layers per window.**
- The **plate technique is cheaper than backdrop-filter**: the plate image is static, frost is pre-baked, and the displacement filter re-runs only when the plate layer is invalidated. Keep the plate on its own layer (`contain: strict`) so icon hover animation doesn't repaint it. Verify with DevTools → Rendering → Paint flashing [Cove, to verify].
- **Idle = 0%.** Nothing on Cove glass should animate when the pointer is away. No rAF loops, no animated wallpaper sampling. Park any rAF when idle and cap DPR at 2 (Na2H2P2O7 pitfalls).
- **Always-on-top transparent windows:** Tauri `transparent: true` + `html, body { background: transparent }` (watch `:root` background rules overriding `html`, per tauri.ninja). WebView2 Runtime 154 has an open bug where `DefaultBackgroundColor` alpha 0 paints an opaque theme background (https://github.com/MicrosoftEdge/WebView2Feedback/issues/5752). Long-lived transparent windows on Win 11 can turn black after display changes (https://github.com/tauri-apps/tauri/issues/16341). **Re-apply transparency on `WM_DISPLAYCHANGE` / DPI change** and test after sleep/resume.
- **Wallpaper sync events:** rebuild the plate on wallpaper change (Cove's light/dark schedule, a user change), monitor change, DPI change and taskbar/work-area change. Per-monitor wallpapers mean per-monitor plates.
- **High DPI:** the map is generated in CSS px and `scale` is in CSS px (`primitiveUnits="userSpaceOnUse"`), and it rendered identically at DPR 1 and 1.5 [tested]. Recheck at 125%, 175% and mixed-DPI multi-monitor in WebView2.
- `prefers-reduced-transparency` follows Windows **Settings → Personalization → Colors → Transparency effects**. Chromium wired it into NativeThemeWin, shipped in Chrome 118 (https://developer.chrome.com/blog/css-prefers-reduced-transparency; Chromium commit "Plumb prefers-reduced-transparency for Windows", issue 40260615). WebView2 should inherit this. Verify on device, and also read the setting in Rust (`HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\EnableTransparency`) as a belt-and-braces flag on `<html data-transparency="off">`.
- **Battery Saver:** DWM acrylic auto-disables [MS]. Mirror that: when on battery saver, set `data-effects="low"` (frost plate, no refraction).

---

## 6. Accessibility checklist

- [ ] Default glass **passes contrast with no OS help**: 4.5:1 body, 3:1 large text and icons, measured over the busiest part of each wallpaper (light and dark) and over a pure white and a pure black test wallpaper.
- [ ] Reduced transparency → solid Ink / warm white surfaces, the same layout, and the rim kept as a 1px border.
- [ ] `prefers-contrast: more` / `forced-colors` → system colours, a visible 1px border, no plate.
- [ ] Reduced motion → crossfades, no springs or magnification bounce. The dock still shows hover state.
- [ ] **Controls must read as controls with glass off** (Armitanemati anti-pattern #21).
- [ ] Hit targets ≥ 28 px (min 20) [3p macOS], with keyboard focus rings visible on glass (2px teal ring + 2px offset, never removed).
- [ ] If legibility fails, fix in this order: raise opacity → move it → add contrast (scrim, stronger edge, heavier weight) → remove glass (Armitanemati §13).

---

## 7. Do / Don't

**Do**
- Glass only on the dock, menu bar, Control Centre and transient menus. Settings and content stay Ink.
- Feed real wallpaper pixels into a plate. That is the only path to real lensing in WebView2.
- Concentrate refraction in a thin bezel. Keep the centre identity and text sharp.
- Let the wallpaper give the colour. Teal on one thing per surface.
- Use rim + shadow + tint as one system, tuned per tier and per mode.
- Use springs at damping 1.0, respond on press, keep motion interruptible, and close on the path it opened.
- Flip the menu bar's ink by wallpaper luminance. Keep big panels fixed.
- Reopen Settings on the last pane, with Ctrl+, and instant controls.
- Make the shadow reach fit inside the transparent window. Click-through the empty area.
- Test on the busiest wallpaper, in motion, at 100/150% DPI, light and dark.

**Don't**
- `backdrop-filter` over the desktop in a transparent window (it blurs nothing).
- Glass on glass, or a child of glass with its own fill, border or blur.
- Gradients, glow blobs, mesh backgrounds (stormaref's violet/pink mesh is **not** for Cove: our backdrop is the wallpaper), purple, or full-surface white sheens.
- Rainbow dispersion, `feTurbulence` wobble, or warping the centre.
- Animating blur radius, rebuilding maps on pointer move, or rAF loops at idle.
- Bounce on things that weren't flicked. Motion on every hover. Blocking input during animation.
- Glass as the only affordance, or text under 14 px at < 500 weight on Regular glass.
- Tinting glass with brand colour, or using solid opaque fills that "break the visual character" [Apple 219].
- Copying Apple assets: no SF Pro, no SF Symbols, no Apple wallpapers. "Cove Glass" is the name (05-research).

---

## 8. Skill repos (cloned in `/workspace/work/design-skills/`)

Newly cloned for this guide (`git clone --depth 1`, 2026-11-03):

| Folder | Repo | Best for |
|---|---|---|
| `Na2H2P2O7_liquid-glass-skill` | https://github.com/Na2H2P2O7/liquid-glass-skill | **Desktop shells**: the "WebView can't see the desktop, so feed the wallpaper as a texture" architecture, a WebGL renderer (`assets/glass.js`) with pressed-glass deformation and adaptive ink, plus a hard-won pitfalls list (fp16 banding, blur ghosting, idle parking). Closest match to Cove's situation. |
| `xinlingfeiwu_liquid-glass-design-skill` | https://github.com/xinlingfeiwu/liquid-glass-design-skill | **Numbers and QA**: `references/golden-glass-style.md` (production vs showcase defaults, inverse-lens map, rejection criteria), an Electron guide, a displacement-map generator and a geometry checker. |
| `stormaref_LiquidGlassSkill` | https://github.com/stormaref/LiquidGlassSkill | **Rules and a portable refraction port**: the four rules (backdrop first, colourless glass, children paint nothing, gate by engine), tiers (chip/panel/menu/modal), and the exact map formulas with bias and filter-region fixes. Ignore its mesh background. |
| `Armitanemati_liquid-glass-claude-skill` | https://github.com/Armitanemati/liquid-glass-claude-skill | **Design reasoning and accessibility**: material tiers with opacity and blur ranges, contrast maths, performance measurements, backdrop-root and mask gotchas, and an anti-pattern table. |

Already present (from earlier work): `emilkowalski_skills` (springs, interruptible motion, respond on press; best for motion), `dickwu_apple-design-skill` (offline HIG copy in `references/hig/*.md`, Tauri-aware reviewer; best for HIG lookups), `naplesblue_apple-design-skill` (restraint: unified surfaces, glass only where layers overlap).

Also worth knowing (not cloned): liquidGL (WebGPU/WebGL, page snapshot) https://github.com/naughtyduk/liquidGL; rdev/liquid-glass-react (SVG, Chromium full effect) https://github.com/rdev/liquid-glass-react; YordiLorenzo/liquid-glass-skills (native SwiftUI/AppKit, for API vocabulary only) https://github.com/YordiLorenzo/liquid-glass-skills; m2-md WebGL2 IOR/dispersion comparison https://github.com/m2-md/liquid-glass-refraction-shader. If Cove ever wants pressed-glass deformation or metaball merging, the WebGL route (Na2H2P2O7 `glass.js`, fed the same wallpaper plate) is the upgrade path.

## 9. Sources

Apple HIG (fetched via the HIG JSON, 2026-11-03): Materials https://developer.apple.com/design/human-interface-guidelines/materials · The menu bar https://developer.apple.com/design/human-interface-guidelines/the-menu-bar · Dock menus https://developer.apple.com/design/human-interface-guidelines/dock-menus · Motion https://developer.apple.com/design/human-interface-guidelines/motion · Settings https://developer.apple.com/design/human-interface-guidelines/settings · WWDC25 219 / 356 via `docs/research/05-liquid-glass-design.md`.
Articles: kube.io, "Liquid Glass in the Browser: Refraction with CSS and SVG" https://kube.io/blog/liquid-glass-css-svg/ · Josh W. Comeau, "Next-level frosted glass with backdrop-filter" https://www.joshwcomeau.com/css/backdrop-filter/ · NN/g, "Liquid Glass Is Cracked" https://www.nngroup.com/articles/liquid-glass · Microsoft Acrylic https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic · Chrome, prefers-reduced-transparency https://developer.chrome.com/blog/css-prefers-reduced-transparency · MDN backdrop-filter https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/backdrop-filter.
WebView2 / Tauri: https://github.com/tauri-apps/window-vibrancy · https://tauri.ninja/recipe.php?id=win-008 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/4945 · https://github.com/tauri-apps/tauri/issues/15512 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5752 · https://github.com/tauri-apps/tauri/issues/16341.
