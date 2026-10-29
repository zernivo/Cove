import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { calculate } from "./calc";

interface Item { kind: "app" | "file" | "setting" | "action" | "calc"; name: string; subtitle: string; target: string }

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
  return Promise.resolve((cmd === "search_index" ? MOCK : null) as T);
}

const q = document.getElementById("q") as HTMLInputElement;
const list = document.getElementById("results") as HTMLUListElement;
const tabs = [...document.querySelectorAll<HTMLButtonElement>("#tabs button")];

let index: Item[] = [];
let shown: Item[] = [];
let sel = 0;
let tab = "all";

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

function compute(): void {
  const query = q.value.trim().toLowerCase();
  let items: Item[];
  if (!query) {
    // Empty: what you use most, then recent files.
    const used = index.filter((it) => usage[key(it)] && inTab(it)).sort((a, b) => usage[key(b)] - usage[key(a)]);
    const recent = index.filter((it) => it.kind === "file" && !usage[key(it)] && inTab(it));
    items = [...used, ...recent].slice(0, 30);
    if (!items.length) items = index.filter(inTab).slice(0, 30);
  } else {
    items = index
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

const GLYPH: Record<Item["kind"], string> = { app: "", file: "📄", setting: "⚙", action: "⏻", calc: "=" };
const LABEL: Record<Item["kind"], string> = { app: "App", file: "File", setting: "Settings", action: "Action", calc: "Calculator" };

function render(): void {
  list.innerHTML = "";
  if (!shown.length) {
    list.innerHTML = `<li class="empty">${index.length ? "No matches" : "Loading your apps…"}</li>`;
    return;
  }
  shown.forEach((it, i) => {
    const li = document.createElement("li");
    li.className = `row${i === sel ? " sel" : ""}`;
    const glyph = it.kind === "app" ? (it.name.trim()[0] || "?").toUpperCase() : GLYPH[it.kind];
    li.innerHTML = `<span class="tile k-${it.kind}"></span><span class="txt"><b></b><small></small></span><span class="tag"></span>`;
    li.querySelector(".tile")!.textContent = glyph;
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
  compute();
}
tabs.forEach((b) => b.addEventListener("click", () => { setTab(b.dataset.tab!); q.focus(); }));

q.addEventListener("input", compute);
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

async function reload(): Promise<void> {
  index = await invoke<Item[]>("search_index");
  compute();
  // Apps load in the background the first time; pick them up shortly.
  if (!index.some((i) => i.kind === "app")) setTimeout(reload, 1500);
}

function opened(): void {
  q.value = "";
  setTab("all");
  q.focus();
  reload();
}

if (inTauri) listen("search-open", opened);
opened();
