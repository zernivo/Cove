import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface TopState { app: string; battery: { percent: number; charging: boolean } | null }

const inTauri = "__TAURI_INTERNALS__" in window;
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  return Promise.resolve((cmd === "topbar_state" ? { app: "Google Chrome", battery: { percent: 53, charging: false } } : null) as T);
}
const $ = (id: string) => document.getElementById(id)!;

// The menu needs room below the 30px bar, so the window grows while it's open.
async function setTall(tall: boolean): Promise<void> {
  if (!inTauri) return;
  const w = getCurrentWindow();
  const size = await w.innerSize();
  const sf = await w.scaleFactor();
  const { PhysicalSize } = await import("@tauri-apps/api/dpi");
  await w.setSize(new PhysicalSize(size.width, Math.round((tall ? 260 : 30) * sf)));
}

const menu = $("menu");
const mark = $("mark");
async function toggleMenu(open?: boolean): Promise<void> {
  const show = open ?? menu.classList.contains("hidden");
  if (show) await setTall(true);
  menu.classList.toggle("hidden", !show);
  mark.classList.toggle("open", show);
  if (!show) await setTall(false);
}
mark.addEventListener("click", (e) => { e.stopPropagation(); toggleMenu(); });
menu.querySelectorAll<HTMLButtonElement>("button[data-go]").forEach((b) =>
  b.addEventListener("click", async () => { await toggleMenu(false); invoke("topbar_open", { what: b.dataset.go }); }),
);
document.addEventListener("click", (e) => { if (!menu.contains(e.target as Node)) toggleMenu(false); });
window.addEventListener("blur", () => toggleMenu(false));

const go = (id: string, what: string) => $(id).addEventListener("click", () => invoke("topbar_open", { what }));
go("search", "search");
go("control", "control");
go("battery", "battery");
go("clock", "notifications");
document.querySelector(".app")!.addEventListener("click", () => invoke("topbar_open", { what: "cove" }));

function clock(): void {
  const d = new Date();
  const day = d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  $("clock").textContent = `${day}  ${time}`;
}

async function refresh(): Promise<void> {
  const s = await invoke<TopState>("topbar_state");
  if (!s) return;
  $("app").textContent = s.app || "Cove";
  const b = s.battery;
  $("battery").classList.toggle("hidden", !b);
  if (b) {
    $("batt-text").textContent = `${b.percent}%`;
    const f = $("batt-fill");
    f.style.width = `${Math.max(2, Math.round((b.percent / 100) * 17))}px`;
    f.className = `fill${b.charging ? " charging" : b.percent <= 20 ? " low" : ""}`;
    $("battery").title = b.charging ? `Charging, ${b.percent}%` : `Battery ${b.percent}%`;
  }
}

clock();
refresh();
setInterval(clock, 1000);
setInterval(refresh, 1000);
