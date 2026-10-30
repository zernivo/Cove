import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface SwitchApp { name: string; exe: string; windows: number[]; icon: string | null }
interface SwitchState { apps: SwitchApp[]; sel: number }

const inTauri = "__TAURI_INTERNALS__" in window;
const panel = document.getElementById("panel")!;

function render(s: SwitchState): void {
  panel.innerHTML = "";
  s.apps.forEach((a, i) => {
    const el = document.createElement("div");
    el.className = `app${i === s.sel ? " sel" : ""}`;
    if (a.icon) el.appendChild(Object.assign(document.createElement("img"), { src: a.icon, alt: "" }));
    else el.appendChild(Object.assign(document.createElement("div"), { className: "letter", textContent: (a.name.trim()[0] || "?").toUpperCase() }));
    el.appendChild(Object.assign(document.createElement("div"), { className: "name", textContent: a.name }));
    if (a.windows.length > 1) el.appendChild(Object.assign(document.createElement("span"), { className: "count", textContent: String(a.windows.length) }));
    el.addEventListener("click", () => { if (inTauri) tauriInvoke("switcher_pick", { index: i }); });
    panel.appendChild(el);
  });
}

if (inTauri) listen<SwitchState>("switch-state", (e) => render(e.payload));
else render({
  sel: 1,
  apps: ["Microsoft Edge", "Code", "File Explorer", "Spotify", "Terminal"].map((n, i) => ({ name: n, exe: n, windows: Array(i % 3 + 1).fill(i), icon: null })),
});
