// Mac window buttons overlay (ui/traffic.html). Cove places this tiny
// window over the left of the focused window's title bar and tells the page
// when to fade in and out; clicks go back to Cove, which posts the window's
// own WM_SYSCOMMAND (close, minimise, zoom). Alt+click on zoom asks for
// Windows' Snap Layouts.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TrafficMsg { show: boolean; fade: number; min: boolean; max: boolean; zoomed: boolean }

const inTauri = "__TAURI_INTERNALS__" in window;
const lights = document.getElementById("lights") as HTMLElement;
const dots = Array.from(document.querySelectorAll<HTMLButtonElement>(".dot"));
const reduce = matchMedia("(prefers-reduced-motion: reduce)");

function apply(m: TrafficMsg): void {
  lights.style.setProperty("--fade", `${reduce.matches ? 0 : m.fade}ms`);
  dots[1].classList.toggle("off", !m.min);
  dots[2].classList.toggle("off", !m.max);
  dots[2].setAttribute("aria-label", m.zoomed ? "Restore" : "Zoom");
  if (!m.show) {
    lights.classList.remove("hover");
    dots.forEach((d) => d.classList.remove("down"));
  }
  lights.classList.toggle("on", m.show);
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

for (const d of dots) {
  // Pressed state shows on pointer-down; the action fires on release over
  // the same dot (a Mac button can be cancelled by sliding off it).
  d.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || d.classList.contains("off")) return;
    d.classList.add("down");
    d.setPointerCapture(e.pointerId);
  });
  d.addEventListener("pointerup", (e) => {
    const was = d.classList.contains("down");
    d.classList.remove("down");
    if (!was) return;
    const r = d.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
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
  // Browser preview / render harness: ?state=rest|hover|pressed|off.
  const q = new URLSearchParams(location.search);
  const state = q.get("state") ?? "rest";
  apply({ show: true, fade: 0, min: state !== "off", max: state !== "off", zoomed: false });
  if (state === "hover" || state === "pressed") lights.classList.add("hover");
  if (state === "pressed") dots[Number(q.get("dot") ?? 0)].classList.add("down");
}
