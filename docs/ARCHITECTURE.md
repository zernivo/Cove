# Architecture

```
Cove/
├─ src-tauri/            Rust core + Tauri 2 shell
│  └─ src/
│     ├─ main.rs         app entry, Tauri commands
│     ├─ journal.rs      change journal: record previous values, restore in reverse
│     └─ tweaks/         one module per setting group (Calm preset etc.)
├─ ui/                   web UI (Vite + TypeScript), Cove Glass tokens
├─ brand/                logo
└─ docs/                 vision, safety, research
```

## Modules (planned)
| Module | Responsibility | Windows APIs |
|---|---|---|
| `journal` | Every change goes through here: previous value, new value, timestamp. `restore_all()` reverts in reverse order | none (pure Rust, JSON on disk) |
| `tweaks` | `Tweak` trait: `id`, `describe`, `apply`, `revert`. Calm preset = a list of tweaks | Registry (HKCU, documented keys), `SystemParametersInfo` |
| `hotkeys` | Mac-style shortcuts | `RegisterHotKey`, `PostMessage(WM_CLOSE)`, `ShowWindow` |
| `appbar` | Dock + top bar reserve screen space | `SHAppBarMessage` |
| `search` | Launcher index + actions | Shell APIs, Windows Search |
| `tour` | Onboarding state | none |

## Performance budget
- About 0% CPU at idle: no polling loops. React to window events (`SetWinEventHook`).
- About 150 MB of memory total: one shared WebView2 environment, at most 2–3 always-on windows, everything else created when needed.
- Animations: compositor-friendly (transform/opacity), following Windows' "Animation effects" setting.

## Visual system
Cove Glass tokens live in `ui/src/tokens.css` (from docs/research/05). Glass is used only for floating controls. Refraction is off by default.

## Fallback
If idle memory can't stay within budget on Tauri, the UI moves to WinUI 3 and the Rust core stays.
