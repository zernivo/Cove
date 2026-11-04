// Cove's intro, in its own full-screen Ink window ("intro"). The app shows
// this window at startup before the dock, the menu bar and Settings, and
// asks it to play: the full voxel build once per installed build, the ~1 s
// logo on every later launch. Click or any key skips. When it ends the page
// tells the app (intro_done), which closes the window and reveals Cove.
//
// Browser preview: intro.html?full=1&t=2.4 draws one frame (for checks).
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { drawIntro, introLength } from "./intro-core";

const inTauri = "__TAURI_INTERNALS__" in window;
const q = new URLSearchParams(location.search);
const cv = document.getElementById("cv") as HTMLCanvasElement;
const g = cv.getContext("2d");

function size(): void {
  const dpr = window.devicePixelRatio || 1;
  const w = Math.round(innerWidth * dpr), h = Math.round(innerHeight * dpr);
  if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h; }
  g?.setTransform(dpr, 0, 0, dpr, 0, 0);
}

async function run(): Promise<void> {
  // The app decides which intro (and logs it); the preview takes ?full=.
  const full = inTauri ? await tauriInvoke<boolean>("intro_begin").catch(() => false) : q.get("full") !== "0";
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const font = getComputedStyle(document.documentElement).getPropertyValue("--font-brand").trim() || "Segoe UI";
  const opts = { full, calm, font };
  const total = introLength(opts);

  const still = q.get("t");
  if (!inTauri && still !== null && g) {
    size();
    drawIntro(g, Number(still), innerWidth, innerHeight, opts);
    document.body.dataset.ready = "1";
    return;
  }

  let skip = false;
  const stop = () => { skip = true; };
  addEventListener("keydown", stop);
  addEventListener("pointerdown", stop);
  // A timer ends it even if frames stall; the app has its own backstop too.
  const cap = total * 1000 + 600;
  await new Promise<void>((done) => {
    let ended = false;
    const end = () => { if (!ended) { ended = true; done(); } };
    setTimeout(end, cap);
    const t0 = performance.now();
    const frame = (now: number) => {
      if (ended) return;
      const t = (now - t0) / 1000;
      if (skip || t >= total || !g) return end();
      size();
      drawIntro(g, t, innerWidth, innerHeight, opts);
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  document.body.classList.add("out");
  await new Promise((r) => setTimeout(r, 280));
  if (inTauri) await tauriInvoke("intro_done", { skipped: skip }).catch(() => {});
}

run();
