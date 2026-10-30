// Cove Glass: puts the chosen theme, light/dark and accent on <html>.
// Every Cove window imports this first so it never flashes the wrong look.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ThemeState { theme: string; mode: string; accent_source: string; accent: string | null }

const inTauri = "__TAURI_INTERNALS__" in window;
const mq = window.matchMedia("(prefers-color-scheme: dark)");
let cur: ThemeState = { theme: "tidepool", mode: "system", accent_source: "cove", accent: null };

function apply(): void {
  const r = document.documentElement;
  r.dataset.theme = cur.theme;
  r.dataset.mode = cur.mode === "system" ? (mq.matches ? "dark" : "light") : cur.mode;
  if (cur.accent) r.style.setProperty("--accent", cur.accent);
  else r.style.removeProperty("--accent");
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
