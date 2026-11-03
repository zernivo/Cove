// Cove intro host: the full voxel build on first launch, a quick mark on every
// later launch, a still mark with Reduce motion. Click or any key skips.
import { drawIntro, introLength } from "./intro-core";

const SEEN = "cove.introSeen";

export async function playIntro(): Promise<void> {
  const full = !localStorage.getItem(SEEN);
  localStorage.setItem(SEEN, "1");
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const font = getComputedStyle(document.documentElement).getPropertyValue("--font-brand").trim() || "Segoe UI";
  const opts = { full, calm, font };
  const host = document.createElement("div");
  host.className = "intro";
  host.setAttribute("aria-hidden", "true");
  const cv = document.createElement("canvas");
  host.appendChild(cv);
  document.body.appendChild(host);
  const g = cv.getContext("2d");
  if (!g) { host.remove(); return; }

  let skip = false;
  const stop = () => { skip = true; };
  addEventListener("keydown", stop);
  host.addEventListener("pointerdown", stop);
  const total = introLength(opts);

  await new Promise<void>((done) => {
    const t0 = performance.now();
    const frame = (now: number) => {
      const t = (now - t0) / 1000;
      if (skip || t >= total) return done();
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
  removeEventListener("keydown", stop);
  host.classList.add("out");
  setTimeout(() => host.remove(), 320);
}
