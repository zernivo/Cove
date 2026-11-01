// Cove intro: the full voxel build of the C on first launch, a quick mark
// every launch after that. Flat Ink background, click or any key skips,
// and Reduce motion shows the mark still for a moment instead.
const INK = "#111215";
const SEEN = "cove.introSeen";
const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;

const clamp = (x: number) => Math.max(0, Math.min(1, x));
const eout = (t: number) => 1 - (1 - clamp(t)) ** 3;
const eback = (t: number, s = 1.2) => { const u = clamp(t) - 1; return 1 + (s + 1) * u ** 3 + s * u ** 2; };

interface Voxel { x: number; y: number; c: string; hi: string; order: number; dx: number; dy: number }

function loadMark(): Promise<HTMLImageElement> {
  return new Promise((ok, no) => { const i = new Image(); i.onload = () => ok(i); i.onerror = no; i.src = "/cove-mark.png"; });
}

/** Sample the mark into voxels (cell px at size px), built round the C from its top end, the cube last. */
function voxels(img: HTMLImageElement, size: number, cell: number): Voxel[] {
  const cv = document.createElement("canvas");
  cv.width = cv.height = size;
  const g = cv.getContext("2d")!;
  g.drawImage(img, 0, 0, size, size);
  const px = g.getImageData(0, 0, size, size).data;
  const out: Voxel[] = [];
  let seed = 7;
  const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const n = Math.floor(size / cell);
  for (let gy = 0; gy < n; gy++) for (let gx = 0; gx < n; gx++) {
    let r = 0, gg = 0, b = 0, k = 0, a = 0;
    for (let y = gy * cell; y < (gy + 1) * cell; y++) for (let x = gx * cell; x < (gx + 1) * cell; x++) {
      const i = (y * size + x) * 4;
      a += px[i + 3];
      if (px[i + 3] > 128) { r += px[i]; gg += px[i + 1]; b += px[i + 2]; k++; }
    }
    if (a / (cell * cell) < 140 || !k) continue;
    r /= k; gg /= k; b /= k;
    const cube = gg > r + 30;
    const x = (gx + 0.5) * cell - size / 2, y = (gy + 0.5) * cell - size / 2;
    const ang = ((Math.atan2(-y, x) * 180) / Math.PI - 25 + 720) % 360;
    const order = cube ? 0.82 + 0.18 * rnd() : (ang / 310) * 0.78 + 0.06 * rnd();
    const dir = Math.atan2(y, x) + (rnd() - 0.5);
    const dist = 90 + 110 * rnd();
    const lift = (v: number) => Math.min(255, Math.round(v + 30));
    out.push({
      x, y, c: `rgb(${r | 0},${gg | 0},${b | 0})`, hi: `rgb(${lift(r)},${lift(gg)},${lift(b)})`,
      order: Math.min(order, 1), dx: Math.cos(dir) * dist, dy: Math.sin(dir) * dist - (30 + 60 * rnd()),
    });
  }
  return out.sort((a, b) => a.order - b.order);
}

function roundRect(g: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  g.beginPath();
  g.roundRect ? g.roundRect(x, y, w, h, r) : g.rect(x, y, w, h);
  g.fill();
}

export async function playIntro(): Promise<void> {
  const first = !localStorage.getItem(SEEN);
  localStorage.setItem(SEEN, "1");
  const host = document.createElement("div");
  host.className = "intro";
  host.setAttribute("aria-hidden", "true");
  const cv = document.createElement("canvas");
  host.appendChild(cv);
  document.body.appendChild(host);
  const dpr = window.devicePixelRatio || 1;
  const fit = () => { cv.width = innerWidth * dpr; cv.height = innerHeight * dpr; };
  fit();
  const g = cv.getContext("2d")!;
  let img: HTMLImageElement;
  try { img = await loadMark(); } catch { host.remove(); return; }

  const total = calm ? 0.9 : first ? 4.8 : 1.3;
  const size = first ? 280 : 160;
  const cell = 10;
  const vox = first && !calm ? voxels(img, size, cell) : [];
  const font = getComputedStyle(document.documentElement).getPropertyValue("--font-brand") || "Segoe UI";
  let skip = false;
  const stop = () => { skip = true; };
  addEventListener("keydown", stop, { once: true });
  host.addEventListener("pointerdown", stop, { once: true });

  await new Promise<void>((done) => {
    const t0 = performance.now();
    const frame = (now: number) => {
      const t = (now - t0) / 1000;
      if (skip || t >= total) return done();
      fit();
      const W = innerWidth, H = innerHeight;
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      g.fillStyle = INK;
      g.fillRect(0, 0, W, H);
      const cx = W / 2;
      if (calm) {
        g.globalAlpha = eout(t / 0.25) * (1 - eout((t - 0.6) / 0.3));
        g.drawImage(img, cx - size / 2, H / 2 - size / 2, size, size);
      } else if (!first) {
        const a = eout(t / 0.35) * (1 - eout((t - 1.0) / 0.3));
        const s = (0.9 + 0.1 * eback(t / 0.45, 1.1)) * size;
        g.globalAlpha = a;
        g.drawImage(img, cx - s / 2, H / 2 - s / 2, s, s);
      } else {
        const cy = H / 2 - 80 * eout((t - 2.75) / 0.7);
        const build = clamp((t - 0.35) / 2.0), swap = eout((t - 2.2) / 0.35), span = 0.35;
        if (swap < 1) {
          g.globalAlpha = 1 - swap;
          for (const v of vox) {
            const lt = (build - v.order * (1 - span)) / span;
            if (lt <= 0) break;
            const k = eout(lt), h = cell * 0.5 * Math.min(eback(lt), 1.15) * 0.94;
            if (h < 0.3) continue;
            const x = cx + v.x + v.dx * (1 - k), y = cy + v.y + v.dy * (1 - k);
            g.fillStyle = v.c;
            roundRect(g, x - h, y - h, h * 2, h * 2, h * 0.22);
            g.fillStyle = v.hi;
            g.fillRect(x - h + 1, y - h + 0.5, h * 2 - 2, 1);
          }
        }
        if (swap > 0) {
          g.globalAlpha = swap;
          g.drawImage(img, cx - size / 2, cy - size / 2, size, size);
        }
        g.textAlign = "center";
        const word = "Cove";
        g.font = `650 58px ${font}`;
        let wx = cx - g.measureText(word).width / 2;
        g.textAlign = "left";
        for (let i = 0; i < word.length; i++) {
          const lt = eout((t - 2.95 - i * 0.08) / 0.45);
          if (lt > 0) {
            g.globalAlpha = lt;
            g.fillStyle = "#f0ece8";
            g.fillText(word[i], wx, H / 2 + 150 + 18 * (1 - lt));
          }
          wx += g.measureText(word[i]).width;
        }
        const ta = eout((t - 3.55) / 0.5);
        if (ta > 0) {
          g.globalAlpha = ta;
          g.textAlign = "center";
          g.font = `400 18px ${font}`;
          g.fillStyle = "#8c8c94";
          g.fillText("Windows, made calm.", cx, H / 2 + 194);
        }
      }
      g.globalAlpha = 1;
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  removeEventListener("keydown", stop);
  host.classList.add("out");
  setTimeout(() => host.remove(), 320);
}
