// Cove intro host: the full voxel build once per installed build, a quick
// (~1 s) logo B on every later launch, calmer with Reduce motion. Click or
// any key skips. The quick mark never holds up input for more than 1.2 s.
import { drawIntro, introLength } from "./intro-core";

const SEEN = "cove.introSeen";

/** `first`: the full build (first launch of this installed build). Decided
 *  by the app, so every new build plays it once; the browser preview falls
 *  back to localStorage. */
export async function playIntro(first?: boolean): Promise<void> {
  const full = first ?? !localStorage.getItem(SEEN);
  localStorage.setItem(SEEN, "1");
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const font = getComputedStyle(document.documentElement).getPropertyValue("--font-brand").trim() || "Segoe UI";
  const opts = { full, calm, font };
  // index.html paints a plain Ink cover first, so the app never flashes in
  // before the intro starts.
  const host = document.getElementById("intro") ?? document.body.appendChild(document.createElement("div"));
  host.className = "intro";
  host.setAttribute("aria-hidden", "true");
  const cv = document.createElement("canvas");
  host.appendChild(cv);
  const g = cv.getContext("2d");
  if (!g) { host.remove(); return; }

  let skip = false;
  const stop = () => { skip = true; };
  addEventListener("keydown", stop);
  host.addEventListener("pointerdown", stop);
  const total = introLength(opts);
  // Quick mark: input is never blocked for more than 1.2 s, whatever the
  // frame rate. Either way a timer ends the intro even if frames stall
  // (a window that isn't visible gets no animation frames).
  const cap = full ? total * 1000 + 600 : 1200;
  const guard = setTimeout(stop, cap);

  await new Promise<void>((done) => {
    const t0 = performance.now();
    let ended = false;
    const end = () => { if (!ended) { ended = true; done(); } };
    setTimeout(end, cap + 50);
    const frame = (now: number) => {
      if (ended) return;
      const t = (now - t0) / 1000;
      if (skip || t >= total) return end();
      const dpr = window.devicePixelRatio || 1;
      const W = innerWidth, H = innerHeight;
      if (cv.width !== Math.round(W * dpr) || cv.height !== Math.round(H * dpr)) {
        cv.width = Math.round(W * dpr);
        cv.height = Math.round(H * dpr);
      }
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      drawIntro(g, t, W, H, opts);
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  clearTimeout(guard);
  removeEventListener("keydown", stop);
  host.classList.add("out");
  setTimeout(() => host.remove(), 320);
}
