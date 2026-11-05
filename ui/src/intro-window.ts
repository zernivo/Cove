// Cove's intro, in its own full-screen Ink window ("intro"). The app sizes
// this (still hidden) window over the main display at startup and asks the
// page which intro to play: the full "Blocks settle" build once per installed
// build, the ~1 s mark on every later launch. The page renders all of its
// sprites first and only then tells the app it is ready (intro_ready); the
// app shows the window, the page paints two warm-up frames, and only then
// starts the clock, so the first drops never hitch. Click or any key skips.
// When it ends the page tells the app (intro_done), which hides the window
// and reveals Cove. The app keeps its own backstop timers.
//
// Browser preview: intro.html?full=1&t=2.4 draws one frame (for checks).
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { IntroScene, INK } from "./intro-core";

const inTauri = "__TAURI_INTERNALS__" in window;
const q = new URLSearchParams(location.search);
const cv = document.getElementById("cv") as HTMLCanvasElement;
const FONT = 'Inter, "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif';

// Opaque, low-latency canvas: no compositing against the page, no extra buffer.
const g = (cv.getContext("2d", { alpha: false, desynchronized: true }) || cv.getContext("2d")) as CanvasRenderingContext2D | null;

const raf = () => new Promise<number>((r) => requestAnimationFrame(r));
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Canvas sized to the window at device pixels. Returns true when it changed. */
function size(): boolean {
  const dpr = window.devicePixelRatio || 1;
  const w = Math.max(1, Math.round(innerWidth * dpr)), h = Math.max(1, Math.round(innerHeight * dpr));
  if (cv.width === w && cv.height === h) return false;
  cv.width = w; cv.height = h;
  return true;
}

async function fontsReady(): Promise<void> {
  // The wordmark font ships with the app (a tiny subset); never wait long for it.
  const load = Promise.all([document.fonts.load('600 60px "Inter"'), document.fonts.load('400 25px "Inter"')]).catch(() => {});
  await Promise.race([load, sleep(1200)]);
}

async function run(): Promise<void> {
  if (!g) { if (inTauri) await tauriInvoke("intro_done", { skipped: false }).catch(() => {}); return; }
  // The app decides which intro (and logs it); the preview takes ?full=.
  const full = inTauri ? await tauriInvoke<boolean>("intro_begin").catch(() => false) : q.get("full") !== "0";
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const scene = new IntroScene({ full, calm, font: FONT });
  const total = scene.length;

  await fontsReady();
  size();
  scene.build(cv.width, cv.height, window.devicePixelRatio || 1);
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.fillStyle = INK;
  g.fillRect(0, 0, cv.width, cv.height);

  const still = q.get("t");
  if (!inTauri && still !== null) {
    scene.draw(g, Number(still));
    // Harnesses step frames with this (recording, checks).
    (window as unknown as { coveIntroDraw: (t: number) => void }).coveIntroDraw = (t) => scene.draw(g, t);
    document.body.dataset.ready = "1";
    return;
  }

  // Sprites are ready: let the app show the window, then wait until it is
  // really on screen and two frames have painted before the clock starts.
  if (inTauri) await tauriInvoke("intro_ready").catch(() => {});
  if (document.visibilityState !== "visible") {
    await Promise.race([new Promise<void>((r) => document.addEventListener("visibilitychange", () => r(), { once: true })), sleep(800)]);
  }
  if (size()) scene.build(cv.width, cv.height, window.devicePixelRatio || 1); // the app may have resized us
  scene.warm(g);
  scene.draw(g, 0);
  await raf();
  scene.warm(g);
  scene.draw(g, 0);
  await raf();

  let skip = false, hidden = false, resized = false;
  const stop = () => { skip = true; };
  addEventListener("keydown", stop);
  addEventListener("pointerdown", stop);
  addEventListener("resize", () => { resized = true; });
  // Hidden mid-play (another desktop, a lock screen): skip to the end.
  document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") hidden = true; });
  document.body.dataset.ready = "1";

  // A timer ends it even if frames stall; the app has its own backstop too.
  await new Promise<void>((done) => {
    let ended = false;
    const end = () => { if (!ended) { ended = true; done(); } };
    setTimeout(end, total * 1000 + 600);
    const t0 = performance.now();
    const frame = (now: number) => {
      if (ended) return;
      // Time-based: a dropped frame never slows the animation down.
      const t = (Math.max(now, t0) - t0) / 1000;
      if (skip || hidden || t >= total) return end();
      if (resized) { resized = false; if (size()) scene.build(cv.width, cv.height, window.devicePixelRatio || 1); }
      scene.draw(g, t);
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  // The full intro ends on the name: fade it out. The short one already ended on Ink.
  if (full && !hidden) {
    document.body.classList.add("out");
    await sleep(280);
  }
  if (inTauri) await tauriInvoke("intro_done", { skipped: skip }).catch(() => {});
}

run();
