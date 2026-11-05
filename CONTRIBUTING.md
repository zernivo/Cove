# Contributing to Cove

Thanks for helping make Windows calm. Cove is a small Rust + Tauri 2 app with a
TypeScript UI. This page covers building, checking, CI, previews, the design
rules and the safety rules every change has to keep.

## Build

Prerequisites: Windows 11 (x64), Rust stable (MSVC toolchain), Node 20+, and the
WebView2 runtime (built into Windows 11).

```bash
git clone https://github.com/zernivo/Cove.git
cd Cove
npm install
npm run tauri dev      # run Cove from source, with hot reload for the UI
npm run tauri build    # build the NSIS and MSI installers
```

The UI also runs in a plain browser with a fake backend, which is handy for
design work: `npm run dev`, then open `http://localhost:5173/` (Settings),
`/popover.html?k=cc` (Control Centre), `/popover.html?k=cove` (a menu) or
`/search.html`.

## Before you push

Run all three. CI runs the same checks on Windows.

```bash
bash notes/windows-check.sh   # type-checks the Rust core for Windows (works from Linux/macOS too)
npx tsc --noEmit              # type-checks the UI
npx vite build                # builds the UI
npm run check:size            # UI bundle size budget (after the build)
```

`notes/windows-check.sh` cross-checks `src-tauri` for `x86_64-pc-windows-msvc`
without linking, so you can catch Windows-only compile errors on any machine.
It installs what it needs (rustup target, clang, llvm) on first run.

## CI

`.github/workflows/ci.yml` runs on every pull request and on `main`, on
`windows-latest`:

1. `npm install`, `npx tsc --noEmit`, `npm run build`
2. **UI bundle size budget** (`scripts/check-bundle-size.mjs`): the JS, CSS and
   HTML Vite emits must stay under the budgets at the top of that script
   (total, gzipped total, and per file). Raise a budget only on purpose, in the
   change that needs it, and say why in the PR.
3. `cargo check` and `cargo test` for `src-tauri`.

A pull request merges when CI is green. We squash-merge, so the PR title
becomes the commit message on `main`: write it as what changed for the user.

## Preview builds

To try `main` on a real PC without cutting a release, run the **Preview build**
workflow (Actions → Preview build → Run workflow). It publishes a pre-release
named `preview` with `Cove-preview-<sha>-setup.exe`. It is never marked latest,
so the in-app updater ignores it. Releases are cut by pushing a `v*` tag; the
**Release** workflow builds and publishes the installers, and the maintainer
then adds the update signature with Cove's update key, which never lives on GitHub.

## Design rules (summary)

The full guide is `docs/design/GLASS_AND_APPLE_GUIDE.md`; read it before any
glass or motion work.

- **Calm first.** Restraint over decoration. No gradients, glow blobs or purple.
  One unified surface rather than stacks of cards; glass only where layers overlap.
- **Graphite for state.** Selection, highlights, focus rings and "on" states are
  graphite (a soft white glass pill with an edge sheen, white text; dark glass
  in light mode). Never the logo's teal (#2cc9b9) or its face colours.
- **Ink.** Cove's own windows sit on Ink `#111215`. Text on glass needs a dark
  tint of at least 0.59 behind it.
- **Liquid Glass, done honestly.** A transparent WebView2 window cannot blur
  what is behind it, so never rely on CSS `backdrop-filter` over the desktop.
  The dock uses the wallpaper plate (a pre-blurred slice of the wallpaper) with
  refraction on the edge band only; panels over apps (Control Centre, Search,
  menus) use Windows acrylic plus a CSS tint and rim.
- **Motion.** Springs that settle without overshoot by default, interruptible,
  responding on pointer-down; enter and exit along the same path.
- **Real states.** Real loading states (skeletons the size of the real thing),
  no placeholders like "…", no fake toggles. Controls apply instantly; no Apply buttons.
- **No Windows title bars** on Cove's windows; Cove draws its own.
- **Accessibility is part of done:** visible graphite focus rings, a sensible tab
  order, ARIA roles and names (menus, menu items, switches, radio groups),
  `prefers-reduced-motion` and `prefers-reduced-transparency` honoured, text
  contrast of at least 4.5:1, click targets of at least 32 DIP.
- **Show mockups first** for any major UI change, and let the maintainer choose.

## Safety rules

Cove must never break Windows or weaken its security. Full text:
[docs/SAFETY.md](docs/SAFETY.md). In short:

- **No injection, no patching.** No DLL injection into Explorer or any other
  process, no uxtheme or system-file patching, no kernel drivers, no edits to
  files under `C:\Windows`.
- **Per-user, documented settings only** (HKCU and public APIs). If Cove can't
  change something safely, it opens the matching Settings page instead.
- **Journal every change** with its previous value, so Restore is exact.
- **Fully reversible.** Turning Cove's desktop off, quitting, or Restore puts
  Windows back exactly as it was. Safe mode pauses everything.
- **No elevation** for normal use. Never touch Defender, SmartScreen, UAC or
  Windows Update.
- **Privacy.** No account, no telemetry. Network use is limited to the
  wallpaper gallery and update checks.
- **No Apple assets.** No Apple fonts, symbols, wallpapers or artwork.

A change that breaks one of these won't be merged, however nice it looks.

## Third-party notices

When you add, remove or bump a dependency, update
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md): run
`cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-msvc`
in `src-tauri` and list the normal (shipped) dependencies with their licences,
plus the npm packages from `package.json`. Only permissive licences
(MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 used unmodified) are accepted.

## Licence

By contributing you agree that your contribution is licensed under the
[MIT licence](LICENSE).
