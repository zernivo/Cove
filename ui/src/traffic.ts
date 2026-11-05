// Mac window buttons overlay (ui/traffic.html). Cove places this window
// exactly over the native caption buttons of the focused window and tells
// the page the title bar's colour; the patch hides the Windows buttons and
// the three dots replace them. Clicks go back to Cove, which posts the
// window's own WM_SYSCOMMAND (close, minimise, zoom). Resting on green (or
// Alt+click) asks for Windows' Snap Layouts.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TrafficMsg { show: boolean; fade: number; min: boolean; max: boolean; zoomed: boolean; color?: string; radius?: number }

const inTauri = "__TAURI_INTERNALS__" in window;
const patch = document.getElementById("patch") as HTMLElement;
const lights = document.getElementById("lights") as HTMLElement;
const dots = Array.from(document.querySelectorAll<HTMLButtonElement>(".dot"));
const reduce = matchMedia("(prefers-reduced-motion: reduce)");

function isLight(hex: string): boolean {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) return false;
  const [r, g, b] = [m[1], m[2], m[3]].map((x) => parseInt(x, 16));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 150;
}

function apply(m: TrafficMsg): void {
  lights.style.setProperty("--fade", `${reduce.matches ? 0 : m.fade}ms`);
  if (m.color) {
    patch.style.setProperty("--bar", m.color);
    patch.classList.toggle("light", isLight(m.color));
  }
  patch.style.setProperty("--r", `${m.radius ?? 0}px`);
  dots[1].classList.toggle("off", !m.min);
  dots[2].classList.toggle("off", !m.max);
  dots[2].setAttribute("aria-label", m.zoomed ? "Restore" : "Zoom");
  if (!m.show) {
    lights.classList.remove("hover");
    dots.forEach((d) => d.classList.remove("down"));
    clearTimeout(snapTimer);
  }
  patch.classList.toggle("on", m.show);
}

function send(cmd: string, args: Record<string, unknown>): void {
  if (inTauri) invoke(cmd, args).catch(() => {});
}

lights.addEventListener("pointerenter", () => {
  lights.classList.add("hover");
  send("traffic_hover", { over: true });
});
lights.addEventListener("pointerleave", () => {
  lights.classList.remove("hover");
  dots.forEach((d) => d.classList.remove("down"));
  send("traffic_hover", { over: false });
});

// Resting on green for a moment opens Snap Layouts, as hovering Windows'
// own maximise button does.
let snapTimer = 0;
const zoom = dots[2];
zoom.addEventListener("pointerenter", () => {
  clearTimeout(snapTimer);
  if (zoom.classList.contains("off")) return;
  snapTimer = window.setTimeout(() => send("traffic_click", { action: "snap", alt: false }), 700);
});
zoom.addEventListener("pointerleave", () => clearTimeout(snapTimer));

for (const d of dots) {
  // Pressed state shows on pointer-down; the action fires on release over
  // the same dot (a Mac button can be cancelled by sliding off it).
  d.addEventListener("pointerdown", (e) => {
    clearTimeout(snapTimer);
    if (e.button !== 0 || d.classList.contains("off")) return;
    d.classList.add("down");
    d.setPointerCapture(e.pointerId);
  });
  d.addEventListener("pointerup", (e) => {
    const was = d.classList.contains("down");
    d.classList.remove("down");
    if (!was) return;
    const r = d.getBoundingClientRect();
    const pad = 4;
    const inside = e.clientX >= r.left - pad && e.clientX <= r.right + pad && e.clientY >= r.top - pad && e.clientY <= r.bottom + pad;
    if (inside) send("traffic_click", { action: d.dataset.act, alt: e.altKey });
  });
  d.addEventListener("pointercancel", () => d.classList.remove("down"));
}

// No context menu, no focus ring, no text selection on the overlay.
addEventListener("contextmenu", (e) => e.preventDefault());

if (inTauri) {
  listen<TrafficMsg>("traffic", (e) => apply(e.payload));
  // Loaded after Cove already asked to show? Catch up.
  invoke<TrafficMsg | null>("traffic_hello").then((m) => { if (m) apply(m); }).catch(() => {});
} else {
  // Browser preview / render harness: ?state=rest|hover|pressed|off&bar=#hex.
  const q = new URLSearchParams(location.search);
  const state = q.get("state") ?? "rest";
  apply({ show: true, fade: 0, min: state !== "off", max: state !== "off", zoomed: false, color: q.get("bar") ?? "#202020", radius: 8 });
  if (state === "hover" || state === "pressed") lights.classList.add("hover");
  if (state === "pressed") dots[Number(q.get("dot") ?? 0)].classList.add("down");
}
