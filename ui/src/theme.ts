// Cove Glass: puts the chosen theme, light/dark and accent on <html>.
// Every Cove window imports this first so it never flashes the wrong look.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ThemeState { theme: string; mode: string; accent_source: string; accent: string | null }

const inTauri = "__TAURI_INTERNALS__" in window;
const mq = window.matchMedia("(prefers-color-scheme: dark)");
let cur: ThemeState = { theme: "tidepool", mode: "system", accent_source: "cove", accent: null };

/** Black or white text, whichever reads better on the accent (WCAG luminance). */
export function onAccent(hex: string): string {
  const n = parseInt(hex.replace("#", ""), 16);
  const lin = (c: number) => { const v = c / 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
  const L = 0.2126 * lin((n >> 16) & 255) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255);
  return (L + 0.05) / 0.05 > 1.05 / (L + 0.05) ? "#111317" : "#ffffff";
}

function apply(): void {
  const r = document.documentElement;
  r.dataset.theme = cur.theme;
  r.dataset.mode = cur.mode === "system" ? (mq.matches ? "dark" : "light") : cur.mode;
  if (cur.accent) {
    r.style.setProperty("--accent", cur.accent);
    r.style.setProperty("--on-accent", onAccent(cur.accent));
  } else {
    r.style.removeProperty("--accent");
    r.style.removeProperty("--on-accent");
  }
}

export function applyTheme(t: ThemeState): void {
  cur = t;
  apply();
}

apply();
mq.addEventListener("change", apply);
if (inTauri) {
  invoke<ThemeState>("theme_get").then(applyTheme).catch(() => {});
  listen<ThemeState>("theme-changed", (e) => applyTheme(e.payload));
}
