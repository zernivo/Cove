import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { calculate } from "./calc";

interface Item { kind: "app" | "file" | "setting" | "action" | "calc" | "clip"; name: string; subtitle: string; target: string; icon?: string }

const inTauri = "__TAURI_INTERNALS__" in window;
const MOCK: Item[] = [
  { kind: "app", name: "Visual Studio Code", subtitle: "App", target: "code", icon: "/mock/vscode.png" },
  { kind: "app", name: "Visual Studio Installer", subtitle: "App", target: "vsi", icon: "/mock/vsinstaller.png" },
  { kind: "file", name: "Vision statement.docx", subtitle: "Documents", target: "C:\\Users\\Khubaib\\Documents\\Vision statement.docx", icon: "/mock/word.png" },
  { kind: "file", name: "visualiser.py", subtitle: "Desktop", target: "C:\\Users\\Khubaib\\Desktop\\BCS-5C\\COAL\\visualiser.py", icon: "/mock/py.png" },
  { kind: "setting", name: "Adjust visual effects", subtitle: "visual effects animations", target: "ms-settings:easeofaccess-visualeffects" },
  { kind: "app", name: "Google Chrome", subtitle: "App", target: "chrome", icon: "/mock/chrome.png" },
  { kind: "setting", name: "Display", subtitle: "brightness", target: "ms-settings:display" },
  { kind: "action", name: "Lock screen", subtitle: "lock computer", target: "lock" },
];
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const mock: Record<string, unknown> = {
    search_index: MOCK,
    icons_system: { settings: "/mock/settings.png" },
    search_files: [],
    clip_history: ["git commit -m \"fix\"", "https://github.com/zernivo/Cove", "Meeting moved to 3 pm"],
  };
  return Promise.resolve((mock[cmd] ?? null) as T);
}

const q = document.getElementById("q") as HTMLInputElement;
const list = document.getElementById("results") as HTMLUListElement;
const panel = document.getElementById("panel")!;
const scopeEl = document.getElementById("scope")!;
if (inTauri && navigator.userAgent.includes("Windows")) document.documentElement.classList.add("acrylic");

// Scopes replace filter chips: Tab cycles them, or type a prefix
// ("f " files, "s " settings, "c " clipboard).
const SCOPES = ["all", "app", "file", "setting", "clip"] as const;
const SCOPE_NAME: Record<string, string> = { all: "All", app: "Apps", file: "Files", setting: "Settings", clip: "Clipboard" };
const PREFIX: Record<string, string> = { f: "file", s: "setting", c: "clip" };

let index: Item[] = [];
let shown: Item[] = [];
let sel = 0;
let tab = "all";
let deep: Item[] = []; // deeper file matches for the current query
let clips: Item[] = [];
let deepTimer = 0;
let sysIcons: Record<string, string | null> = {};

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

/** The query without a scope prefix, and the scope it asks for. */
function parse(): { query: string; scope: string } {
  const raw = q.value.replace(/^\s+/, "");
  const m = /^([fsc]) (.*)$/i.exec(raw);
  if (m && PREFIX[m[1].toLowerCase()]) return { query: m[2].trim().toLowerCase(), scope: PREFIX[m[1].toLowerCase()] };
  return { query: raw.trim().toLowerCase(), scope: tab };
}
const inScope = (it: Item, scope: string) => scope === "all" || it.kind === scope || (scope === "setting" && it.kind === "action");

function clipItems(query: string): Item[] {
  const items = query ? clips.filter((c) => c.target.toLowerCase().includes(query)) : clips;
  return items.slice(0, 50);
}

/** Friendly names only: never raw ids such as "ms-screenclip---source=HotKey". */
const raw = (it: Item) => /^[a-z][\w.-]*:|---|[{}]|^[A-F0-9]{8}-/i.test(it.name);

function compute(): void {
  const { query, scope } = parse();
  let items: Item[];
  document.body.classList.toggle("clip-tab", scope === "clip");
  if (scope === "clip") {
    items = clipItems(query);
  } else if (!query) {
    items = [];
  } else {
    const known = new Set(index.map(key));
    items = [...index, ...deep.filter((d) => !known.has(key(d)))]
      .filter((it) => inScope(it, scope) && !raw(it))
      .map((it) => [it, score(it, query)] as const)
      .filter(([, s]) => s >= 0)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 30)
      .map(([it]) => it);
    const v = calculate(q.value);
    if (v !== null && scope === "all") {
      const out = +v.toPrecision(12);
      items.unshift({ kind: "calc", name: `= ${out.toLocaleString()}`, subtitle: "Press Enter to copy", target: String(out) });
    }
  }
  document.body.classList.toggle("empty-q", !query && scope !== "clip");
  scopeEl.classList.toggle("hidden", scope === "all");
  scopeEl.textContent = SCOPE_NAME[scope];
  const nxt = SCOPES[(SCOPES.indexOf(tab as typeof SCOPES[number]) + 1) % SCOPES.length];
  document.getElementById("next-scope")!.textContent = SCOPE_NAME[nxt];
  shown = items;
  sel = 0;
  render();
}

const TYPE_BY_EXT: Record<string, string> = {
  doc: "Word document", docx: "Word document", xls: "Excel workbook", xlsx: "Excel workbook", ppt: "PowerPoint presentation", pptx: "PowerPoint presentation",
  pdf: "PDF document", txt: "Text document", md: "Markdown file", py: "Python file", js: "JavaScript file", ts: "TypeScript file", rs: "Rust file", c: "C source file", cpp: "C++ source file",
  java: "Java file", html: "HTML file", css: "CSS file", json: "JSON file", png: "Image", jpg: "Image", jpeg: "Image", gif: "Image", webp: "Image", svg: "Image",
  mp3: "Audio", wav: "Audio", flac: "Audio", mp4: "Video", mkv: "Video", mov: "Video", zip: "ZIP archive", rar: "Archive", "7z": "Archive", exe: "Application", msi: "Installer", lnk: "Shortcut",
};
function typeOf(it: Item): string {
  switch (it.kind) {
    case "app": return "App";
    case "setting": return "Setting";
    case "action": return "Action";
    case "calc": return "Calculator";
    case "clip": return "Clipboard";
    default: {
      const ext = /\.([a-z0-9]+)$/i.exec(it.name)?.[1]?.toLowerCase();
      if (!ext) return "Folder";
      return TYPE_BY_EXT[ext] ?? `${ext.toUpperCase()} file`;
    }
  }
}
/** Grey context after the name: the folder a file is in, a setting's section. */
function contextOf(it: Item): string {
  if (it.kind === "file") {
    const parts = it.target.split(/[\\/]/).filter(Boolean);
    parts.pop();
    const u = parts.findIndex((p) => p.toLowerCase() === "users");
    const rel = u >= 0 ? parts.slice(u + 2) : parts.slice(-3);
    return (rel.length ? rel.slice(-3) : parts.slice(-1)).join(" › ");
  }
  if (it.kind === "setting") return "Settings";
  if (it.kind === "clip") return it.subtitle === "Text" ? "" : it.subtitle;
  return "";
}
const GLYPH: Record<string, string> = {
  file: `<svg viewBox="0 0 16 16"><path d="M4 1.8h5.2L12.4 5v9.2H4z"/><path d="M9 1.8V5.2h3.4"/></svg>`,
  setting: `<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="2.2"/><path d="M8 1.6v1.8M8 12.6v1.8M3.5 3.5l1.3 1.3M11.2 11.2l1.3 1.3M1.6 8h1.8M12.6 8h1.8M3.5 12.5l1.3-1.3M11.2 4.8l1.3-1.3"/></svg>`,
  action: `<svg viewBox="0 0 16 16"><path d="M8 2v6"/><path d="M4.6 4.4a5 5 0 1 0 6.8 0"/></svg>`,
  clip: `<svg viewBox="0 0 16 16"><rect x="3.4" y="2.8" width="9.2" height="11.4" rx="1.6"/><path d="M6 2.8h4v2H6z"/></svg>`,
  app: `<svg viewBox="0 0 16 16"><rect x="2.6" y="2.6" width="10.8" height="10.8" rx="2.6"/></svg>`,
};

function render(): void {
  list.innerHTML = "";
  if (!shown.length) {
    const { query, scope } = parse();
    if (scope === "clip" || query) {
      const msg = scope === "clip" ? (clips.length ? "No matches" : "Nothing copied yet. Text you copy shows up here while Cove runs.") : index.length ? "No results" : "Loading your apps…";
      list.innerHTML = `<li class="empty"></li>`;
      list.firstElementChild!.textContent = msg;
    }
    fit();
    return;
  }
  shown.forEach((it, i) => {
    const li = document.createElement("li");
    li.className = `row${i === sel ? " sel" : ""}`;
    li.innerHTML = `<span class="tile"></span><span class="txt"><b></b><small></small></span><span class="tag"></span>`;
    const tile = li.querySelector(".tile") as HTMLElement;
    const icon = it.icon ?? (it.kind === "setting" ? sysIcons.settings : null);
    if (icon) {
      tile.classList.add("has-icon");
      const img = Object.assign(document.createElement("img"), { src: icon, alt: "" });
      img.addEventListener("error", () => { tile.classList.remove("has-icon"); tile.innerHTML = GLYPH[it.kind] ?? ""; });
      tile.appendChild(img);
    } else if (it.kind === "calc") tile.innerHTML = `<span class="calc">=</span>`;
    else tile.innerHTML = GLYPH[it.kind] ?? GLYPH.app;
    li.querySelector("b")!.textContent = it.name;
    li.querySelector("small")!.textContent = contextOf(it);
    li.querySelector(".tag")!.textContent = typeOf(it);
    li.addEventListener("mousemove", () => { if (sel !== i) { sel = i; mark(); } });
    li.addEventListener("click", () => activate(it, false));
    list.appendChild(li);
  });
  list.children[sel]?.scrollIntoView({ block: "nearest" });
  fit();
}
function mark(): void {
  [...list.children].forEach((li, i) => li.classList.toggle("sel", i === sel));
  list.children[sel]?.scrollIntoView({ block: "nearest" });
}

/** Size (and shape) the window to the panel: field only, or with results. */
let lastH = 0;
function fit(): void {
  const h = Math.ceil(panel.getBoundingClientRect().height);
  if (h === lastH) return;
  lastH = h;
  if (inTauri) invoke("search_fit", { height: h }).catch(() => {});
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
    fit();
  }
}

function setTab(t: string): void {
  tab = t;
  if (t === "clip") loadClips();
  compute();
}

async function loadClips(): Promise<void> {
  const h = (await invoke<string[]>("clip_history")) || [];
  clips = h.map((t) => ({ kind: "clip", name: t.replace(/\s+/g, " ").trim().slice(0, 140), subtitle: t.length > 140 ? `${t.length} characters` : "Text", target: t }));
  if (parse().scope === "clip") compute();
}

function fetchDeep(attempt = 0): void {
  const { query, scope } = parse();
  clearTimeout(deepTimer);
  if (query.length < 2 || scope === "clip") { deep = []; return; }
  deepTimer = window.setTimeout(async () => {
    const got = (await invoke<Item[]>("search_files", { query })) || [];
    if (parse().query !== query) return;
    const prev = shown[sel] ? key(shown[sel]) : "";
    deep = got;
    compute();
    const i = shown.findIndex((s) => key(s) === prev);
    if (i > 0) { sel = i; mark(); }
    if (attempt < 3 && (attempt < 1 || !got.length || got.some((g) => !g.icon))) setTimeout(() => fetchDeep(attempt + 1), 700);
  }, attempt ? 0 : 90);
}

q.addEventListener("input", () => { compute(); fetchDeep(); });
document.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(sel + 1, shown.length - 1); mark(); }
  else if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(sel - 1, 0); mark(); }
  else if ((e.key === "d" || e.key === "D") && e.ctrlKey) {
    e.preventDefault();
    const it = shown[sel];
    if (it && (it.kind === "app" || (it.kind === "file" && it.target.toLowerCase().endsWith(".exe")))) {
      invoke("dock_pin", { name: it.name, target: it.target }).then(() => {
        const small = list.children[sel]?.querySelector("small");
        if (small) small.textContent = "Added to Dock";
      }).catch(() => {});
    }
  }
  else if (e.key === "Enter") { e.preventDefault(); activate(shown[sel], e.ctrlKey); }
  else if (e.key === "Tab") {
    e.preventDefault();
    const n = (SCOPES.indexOf(tab as typeof SCOPES[number]) + (e.shiftKey ? SCOPES.length - 1 : 1)) % SCOPES.length;
    setTab(SCOPES[n]);
  } else if (e.key === "Escape") {
    e.preventDefault();
    if (q.value) { q.value = ""; compute(); } else if (tab !== "all") setTab("all"); else invoke("search_hide");
  }
});

let tries = 0;
async function reload(): Promise<void> {
  const prevSel = shown[sel] ? key(shown[sel]) : "";
  index = await invoke<Item[]>("search_index");
  const keepSel = sel;
  compute();
  if (prevSel && shown[keepSel] && key(shown[keepSel]) === prevSel) { sel = keepSel; mark(); }
  const pending = !index.some((i) => i.kind === "app") || index.some((i) => (i.kind === "app" || i.kind === "file") && !i.icon);
  if (pending && tries++ < 12 && document.hasFocus()) setTimeout(reload, 1200);
}

function opened(): void {
  tries = 0;
  q.value = new URLSearchParams(location.search).get("q") ?? "";
  deep = [];
  tab = "all";
  loadClips();
  q.focus();
  panel.classList.remove("in");
  void panel.offsetWidth;
  panel.classList.add("in");
  invoke<Record<string, string | null>>("icons_system").then((i) => { sysIcons = i ?? {}; render(); }).catch(() => {});
  reload();
  if (inTauri) requestAnimationFrame(() => requestAnimationFrame(() => { invoke("search_shown").catch(() => {}); }));
}

if (inTauri) {
  listen("search-open", opened);
  const syncGlass = () => invoke("glass_tint", { dark: document.documentElement.dataset.mode === "dark" }).catch(() => {});
  syncGlass();
  new MutationObserver(syncGlass).observe(document.documentElement, { attributes: true, attributeFilter: ["data-mode"] });
}
opened();
