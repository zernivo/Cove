import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { calculate } from "./calc";

interface Item { kind: "app" | "file" | "setting" | "action" | "calc" | "clip"; name: string; subtitle: string; target: string; icon?: string }

const inTauri = "__TAURI_INTERNALS__" in window;
const MOCK: Item[] = [
  { kind: "app", name: "Google Chrome", subtitle: "App", target: "chrome" },
  { kind: "app", name: "Visual Studio Code", subtitle: "App", target: "code" },
  { kind: "app", name: "Obsidian", subtitle: "App", target: "obsidian" },
  { kind: "app", name: "Calculator", subtitle: "App", target: "calc" },
  { kind: "file", name: "CA_Assignment_2.pdf", subtitle: "Recent file", target: "C:\\x.pdf" },
  { kind: "setting", name: "Bluetooth & devices", subtitle: "bluetooth headphones", target: "ms-settings:bluetooth" },
  { kind: "setting", name: "Display", subtitle: "brightness", target: "ms-settings:display" },
  { kind: "action", name: "Lock screen", subtitle: "lock computer", target: "lock" },
];
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const mock: Record<string, unknown> = {
    search_index: MOCK,
    search_files: [],
    clip_history: ["git commit -m \"fix\"", "https://github.com/zernivo/Cove", "Meeting moved to 3 pm"],
  };
  return Promise.resolve((mock[cmd] ?? null) as T);
}

const q = document.getElementById("q") as HTMLInputElement;
const list = document.getElementById("results") as HTMLUListElement;
const tabs = [...document.querySelectorAll<HTMLButtonElement>("#tabs button")];

let index: Item[] = [];
let shown: Item[] = [];
let sel = 0;
let tab = "all";
let deep: Item[] = []; // deeper file matches for the current query
let clips: Item[] = [];
let deepTimer = 0;

// ---- learning: what you open most ranks higher ----
const USAGE_KEY = "cove.search.usage";
const usage: Record<string, number> = JSON.parse(localStorage.getItem(USAGE_KEY) || "{}");
const key = (it: Item) => `${it.kind}:${it.target}`;
function bump(it: Item): void {
  usage[key(it)] = (usage[key(it)] || 0) + 1;
  localStorage.setItem(USAGE_KEY, JSON.stringify(usage));
}

// ---- ranking ----
function score(it: Item, query: string): number {
  const name = it.name.toLowerCase();
  let s: number;
  if (name === query) s = 1000;
  else if (name.startsWith(query)) s = 800 - name.length;
  else if (name.split(/[\s\-_.]+/).some((w) => w.startsWith(query))) s = 600 - name.length;
  else if (initials(name).startsWith(query)) s = 550;
  else if (name.includes(query)) s = 400 - name.indexOf(query);
  else if (it.subtitle.toLowerCase().includes(query)) s = 250;
  else if (subsequence(name, query)) s = 120;
  else return -1;
  if (it.kind === "app") s += 40;
  return s + Math.min(usage[key(it)] || 0, 20) * 15;
}
const initials = (n: string) => n.split(/[\s\-_.]+/).map((w) => w[0] || "").join("");
function subsequence(text: string, query: string): boolean {
  let j = 0;
  for (const c of text) if (c === query[j]) j++;
  return j === query.length;
}

const inTab = (it: Item) =>
  tab === "all" || it.kind === tab || (tab === "setting" && it.kind === "action");

function clipItems(query: string): Item[] {
  const items = query ? clips.filter((c) => c.target.toLowerCase().includes(query)) : clips;
  return items.slice(0, 50);
}

function compute(): void {
  const query = q.value.trim().toLowerCase();
  let items: Item[];
  if (tab === "clip") {
    items = clipItems(query);
  } else if (!query) {
    // Empty: what you use most, then recent files.
    const used = index.filter((it) => usage[key(it)] && inTab(it)).sort((a, b) => usage[key(b)] - usage[key(a)]);
    const recent = index.filter((it) => it.kind === "file" && !usage[key(it)] && inTab(it));
    const seen = new Set([...used, ...recent].map(key));
    const rest = index.filter((it) => inTab(it) && !seen.has(key(it)));
    items = [...used, ...recent, ...rest].slice(0, 40);
  } else {
    const known = new Set(index.map(key));
    items = [...index, ...deep.filter((d) => !known.has(key(d)))]
      .filter(inTab)
      .map((it) => [it, score(it, query)] as const)
      .filter(([, s]) => s >= 0)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 40)
      .map(([it]) => it);
    const v = calculate(q.value);
    if (v !== null && (tab === "all")) {
      const out = +v.toPrecision(12);
      items.unshift({ kind: "calc", name: `= ${out.toLocaleString()}`, subtitle: "Press Enter to copy", target: String(out) });
    }
  }
  shown = items;
  sel = 0;
  render();
}

const GLYPH: Record<Item["kind"], string> = { app: "", file: "📄", setting: "⚙", action: "⏻", calc: "=", clip: "📋" };
const LABEL: Record<Item["kind"], string> = { app: "App", file: "File", setting: "Settings", action: "Action", calc: "Calculator", clip: "Clipboard" };

function render(): void {
  list.innerHTML = "";
  if (!shown.length) {
    const msg = tab === "clip"
      ? clips.length ? "No matches" : "Nothing copied yet. Text you copy shows up here (kept only while Cove runs)."
      : index.length ? "No matches" : "Loading your apps…";
    list.innerHTML = `<li class="empty"></li>`;
    list.firstElementChild!.textContent = msg;
    return;
  }
  shown.forEach((it, i) => {
    const li = document.createElement("li");
    li.className = `row${i === sel ? " sel" : ""}`;
    const glyph = it.kind === "app" ? (it.name.trim()[0] || "?").toUpperCase() : GLYPH[it.kind];
    li.innerHTML = `<span class="tile k-${it.kind}"></span><span class="txt"><b></b><small></small></span><span class="tag"></span>`;
    const tile = li.querySelector(".tile") as HTMLElement;
    if (it.icon) {
      tile.classList.add("has-icon");
      const img = document.createElement("img");
      img.src = it.icon;
      img.alt = "";
      tile.appendChild(img);
    } else tile.textContent = glyph;
    li.querySelector("b")!.textContent = it.name;
    li.querySelector("small")!.textContent = it.kind === "setting" || it.kind === "action" ? LABEL[it.kind] : it.subtitle;
    li.querySelector(".tag")!.textContent = i === sel ? "↵" : "";
    li.addEventListener("mousemove", () => { if (sel !== i) { sel = i; render(); } });
    li.addEventListener("click", () => activate(it, false));
    list.appendChild(li);
  });
  list.children[sel]?.scrollIntoView({ block: "nearest" });
}

async function activate(it: Item | undefined, reveal: boolean): Promise<void> {
  if (!it) return;
  if (it.kind === "clip") {
    try { await invoke("clip_use", { text: it.target, paste: !reveal }); } catch (e) { console.error(e); }
    return;
  }
  if (it.kind === "calc") {
    await navigator.clipboard.writeText(it.target).catch(() => {});
    q.value = it.target;
    compute();
    return;
  }
  bump(it);
  try {
    if (reveal && it.kind === "file") await invoke("search_reveal", { target: it.target });
    else await invoke("search_open", { kind: it.kind, target: it.target });
  } catch (e) {
    list.innerHTML = `<li class="empty"></li>`;
    list.firstElementChild!.textContent = String(e);
  }
}

function setTab(t: string): void {
  tab = t;
  tabs.forEach((b) => b.classList.toggle("on", b.dataset.tab === t));
  document.body.classList.toggle("clip-tab", t === "clip");
  if (t === "clip") loadClips();
  compute();
}
tabs.forEach((b) => b.addEventListener("click", () => { setTab(b.dataset.tab!); q.focus(); }));

async function loadClips(): Promise<void> {
  const h = (await invoke<string[]>("clip_history")) || [];
  clips = h.map((t) => ({ kind: "clip", name: t.replace(/\s+/g, " ").trim().slice(0, 140), subtitle: t.length > 140 ? `${t.length} characters` : "Text", target: t }));
  if (tab === "clip") compute();
}

function fetchDeep(attempt = 0): void {
  const query = q.value.trim();
  clearTimeout(deepTimer);
  if (query.length < 2 || tab === "clip") { deep = []; return; }
  deepTimer = window.setTimeout(async () => {
    const got = (await invoke<Item[]>("search_files", { query })) || [];
    if (q.value.trim() !== query) return;
    const prev = shown[sel] ? key(shown[sel]) : "";
    deep = got;
    compute();
    const i = shown.findIndex((s) => key(s) === prev);
    if (i > 0) { sel = i; render(); }
    // Index or icons still warming up: look again shortly.
    if (attempt < 3 && (!got.length || got.some((g) => !g.icon))) setTimeout(() => fetchDeep(attempt + 1), 700);
  }, attempt ? 0 : 90);
}

q.addEventListener("input", () => { compute(); fetchDeep(); });
document.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(sel + 1, shown.length - 1); render(); }
  else if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(sel - 1, 0); render(); }
  else if (e.key === "Enter") { e.preventDefault(); activate(shown[sel], e.ctrlKey); }
  else if (e.key === "Tab") {
    e.preventDefault();
    const order = tabs.map((b) => b.dataset.tab!);
    const n = (order.indexOf(tab) + (e.shiftKey ? order.length - 1 : 1)) % order.length;
    setTab(order[n]);
  } else if (e.key === "Escape") {
    e.preventDefault();
    if (q.value) { q.value = ""; compute(); } else invoke("search_hide");
  }
});

let tries = 0;
async function reload(): Promise<void> {
  const prevSel = shown[sel] ? key(shown[sel]) : "";
  index = await invoke<Item[]>("search_index");
  const keepSel = sel;
  compute();
  if (prevSel && shown[keepSel] && key(shown[keepSel]) === prevSel) { sel = keepSel; render(); }
  // Apps and their icons load in the background; pick them up shortly.
  const pending = !index.some((i) => i.kind === "app") || index.some((i) => (i.kind === "app" || i.kind === "file") && !i.icon);
  if (pending && tries++ < 12 && document.hasFocus()) setTimeout(reload, 1200);
}

function opened(): void {
  tries = 0;
  q.value = "";
  deep = [];
  setTab("all");
  loadClips();
  q.focus();
  reload();
}

if (inTauri) listen("search-open", opened);
opened();
