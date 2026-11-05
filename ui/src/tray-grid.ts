// Control Centre's "Other apps" section: other apps' tray icons (Discord,
// Steam, OneDrive...) in a grid, 6 to a row, 20 DIP icons. Hover names the
// icon (the app's own tooltip, shown inside the panel so the shaped window
// never clips it); a left click, a right click and a double click go to the
// app exactly as the taskbar would send them, so its own menu or window
// opens. "Windows tray" opens Windows' own overflow for anything missing.
import "./tray.css";
import type { TrayItem, TrayStatus } from "./tray";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

export interface TrayGrid {
  /** Re-read the icons; resolves once the section is drawn (call `onResize` after). */
  refresh(): Promise<void>;
  /** Draw from the last known state right away (no wait). */
  paintCached(): void;
}

const APP_GLYPH = `<svg viewBox="0 0 16 16"><rect x="2.5" y="2.5" width="11" height="11" rx="3"/></svg>`;

export function mountTrayGrid(root: HTMLElement, invoke: Invoke, opts: { onResize: () => void; onAct?: () => void }): TrayGrid {
  root.classList.add("module", "tray-mod");
  root.innerHTML = `
    <div class="head"><b>Other apps</b><button class="link" data-act="native" title="Open Windows' own tray">Windows tray</button></div>
    <div class="tray-grid" role="toolbar" aria-label="Other apps' tray icons"></div>
    <p class="note tray-empty hidden"></p>
    <div class="tray-tip hidden" role="tooltip"></div>`;
  const grid = root.querySelector<HTMLElement>(".tray-grid")!;
  const empty = root.querySelector<HTMLElement>(".tray-empty")!;
  const tip = root.querySelector<HTMLElement>(".tray-tip")!;
  let last: TrayStatus | null = null;
  let sig = "";
  let tipTimer = 0;

  root.querySelector<HTMLElement>('[data-act="native"]')!.addEventListener("click", () => {
    opts.onAct?.();
    invoke("tray_open_windows").catch(() => {});
  });

  const showTip = (cell: HTMLElement, it: TrayItem) => {
    clearTimeout(tipTimer);
    tipTimer = window.setTimeout(() => {
      tip.textContent = it.tip.split("\n").map((l) => l.trim()).filter(Boolean).slice(0, 2).join(" · ") || it.name;
      tip.classList.remove("hidden");
      // Above the icon, kept inside the section.
      const r = root.getBoundingClientRect();
      const c = cell.getBoundingClientRect();
      const tw = tip.offsetWidth;
      const x = Math.min(Math.max(6, c.left - r.left + c.width / 2 - tw / 2), r.width - tw - 6);
      tip.style.left = `${x}px`;
      tip.style.top = `${c.top - r.top - tip.offsetHeight - 4}px`;
    }, 350);
  };
  const hideTip = () => { clearTimeout(tipTimer); tip.classList.add("hidden"); };

  const send = (it: TrayItem, how: "left" | "right" | "double") => {
    hideTip();
    invoke("tray_click", { id: it.id, how }).catch(() => {});
  };

  function draw(s: TrayStatus): void {
    const show = s.mode !== "off";
    root.classList.toggle("hidden", !show);
    if (!show) return;
    const items = s.items;
    const nextSig = JSON.stringify(items.map((i) => [i.id, i.icon, i.tip]));
    if (nextSig !== sig) {
      sig = nextSig;
      grid.innerHTML = "";
      for (const it of items) {
        const b = document.createElement("button");
        b.className = "tray-cell";
        b.setAttribute("aria-label", it.tip || it.name);
        if (it.icon) {
          const img = document.createElement("img");
          img.src = it.icon;
          img.alt = "";
          img.draggable = false;
          img.onerror = () => { img.replaceWith(Object.assign(document.createElement("span"), { className: "tray-glyph", innerHTML: APP_GLYPH })); };
          b.appendChild(img);
        } else {
          b.appendChild(Object.assign(document.createElement("span"), { className: "tray-letter", textContent: (it.name || "?").slice(0, 1).toUpperCase() }));
        }
        b.addEventListener("pointerenter", () => showTip(b, it));
        b.addEventListener("pointerleave", hideTip);
        b.addEventListener("click", (e) => { if (e.detail <= 1) send(it, "left"); });
        b.addEventListener("dblclick", () => send(it, "double"));
        b.addEventListener("contextmenu", (e) => { e.preventDefault(); send(it, "right"); });
        b.addEventListener("keydown", (e) => {
          if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) { e.preventDefault(); send(it, "right"); }
        });
        grid.appendChild(b);
      }
    }
    grid.classList.toggle("hidden", items.length === 0);
    empty.classList.toggle("hidden", items.length > 0);
    empty.textContent = s.note && items.length === 0
      ? s.note
      : "No other apps have tray icons right now.";
  }

  return {
    paintCached() { if (last) draw(last); },
    async refresh() {
      const s = await invoke<TrayStatus>("tray_status").catch(() => null);
      if (!s) { root.classList.add("hidden"); return; }
      last = s;
      draw(s);
    },
  };
}
