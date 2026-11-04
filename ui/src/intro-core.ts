// Cove intro, drawing only (no DOM). A real voxel model of the C, seen
// isometrically: every strip of the C is made of small cubes that drop from
// above, land, and settle with a short bounce, built from the ground up.
// The teal cube falls in last and bounces into the middle, then the seams
// close so the C reads as one solid object, and the name rises in.

export const INK = "#111215";

const N = 12; // cube grid edge, in voxels
const T = 3; // bar thickness, in voxels
const FACE = { top: "#f7dccd", left: "#8c5a64", right: "#c48b82" };
const TEAL = { top: "#a6f2e6", left: "#169487", right: "#2cc9b9" };

interface Vox { x: number; y: number; z: number; land: number; teal?: boolean }

const clamp = (x: number) => Math.max(0, Math.min(1, x));
export const eout = (t: number) => 1 - (1 - clamp(t)) ** 3;

/** The C: a thick ring around the cube's six silhouette edges, open on the right. */
function model(): Vox[] {
  const v: Vox[] = [];
  const near = (a: number, edge: number) => (edge === 0 ? a < T : a >= N - T);
  for (let x = 0; x < N; x++) for (let y = 0; y < N; y++) for (let z = 0; z < N; z++) {
    // Silhouette edges seen from (+1,+1,+1): the ones touching neither the
    // nearest corner (N,N,N) nor the farthest (0,0,0).
    const bars = [
      near(x, N) && near(y, 0), // vertical, screen right  (x=N, y=0)
      near(x, 0) && near(y, N), // vertical, screen left   (x=0, y=N)
      near(x, N) && near(z, 0), // bottom right
      near(y, N) && near(z, 0), // bottom left
      near(x, 0) && near(z, N), // top left
      near(y, 0) && near(z, N), // top right
    ];
    if (!bars.some(Boolean)) continue;
    // Open the C on the right: drop the right vertical bar, except short arms.
    const rightOnly = bars[0] && !bars[2] && !bars[5];
    if (rightOnly) continue;
    if (bars[0] && z >= T && z < N - T) continue;
    v.push({ x, y, z, land: 0 });
  }
  // Build order: from the ground up, a little shuffled so it feels poured, not printed.
  let seed = 11;
  const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  const keyed = v.map((p) => ({ p, k: p.z + rnd() * 2.2 }));
  keyed.sort((a, b) => a.k - b.k);
  keyed.forEach((e, i) => (e.p.land = i / keyed.length));
  // Draw back to front (painter's order for a view from +x +y +z).
  return v.sort((a, b) => a.x + a.y + a.z - (b.x + b.y + b.z) || a.z - b.z);
}

const RING = model();

/** Bouncing-ball settle after landing: two quick hops, then still. tau in seconds. */
function bounce(tau: number, h: number): number {
  if (tau <= 0) return 0;
  const hops = [[0.16, 1], [0.11, 0.35], [0.07, 0.12]];
  let t = tau;
  for (const [d, a] of hops) {
    if (t < d) { const u = t / d; return h * a * 4 * u * (1 - u); }
    t -= d;
  }
  return 0;
}

/**
 * One voxel, drawn crisp to match the sharp-edged logo (option B): corners sit
 * on the device-pixel grid, faces are closed polygons with mitred (never
 * rounded) joins, and each face is outlined in its own colour by a single
 * device pixel so anti-aliasing can't open soft, see-through cracks between
 * neighbouring faces and blocks.
 */
function cube(g: CanvasRenderingContext2D, sx: number, sy: number, s: number, c: typeof FACE, seam: number, squash = 1) {
  const k = g.getTransform().a || 1; // CSS px -> device px
  const px = (v: number) => Math.round(v * k) / k;
  sx = px(sx); sy = px(sy);
  const w = px(s * 0.866), hh = px(s * 0.5), hgt = px(s * squash);
  const top = sy - hgt; // y of the top face's centre
  g.lineJoin = "miter";
  g.miterLimit = 10;
  g.lineCap = "butt";
  g.lineWidth = 1 / k;
  const face = (col: string, pts: [number, number][]) => {
    g.fillStyle = col; g.strokeStyle = col;
    g.beginPath(); g.moveTo(pts[0][0], pts[0][1]);
    for (let i = 1; i < pts.length; i++) g.lineTo(pts[i][0], pts[i][1]);
    g.closePath(); g.fill(); g.stroke();
  };
  face(c.top, [[sx, top - hh], [sx + w, top], [sx, top + hh], [sx - w, top]]);
  face(c.left, [[sx - w, top], [sx, top + hh], [sx, sy + hh], [sx - w, sy]]);
  face(c.right, [[sx + w, top], [sx, top + hh], [sx, sy + hh], [sx + w, sy]]);
  if (seam > 0.01) {
    g.strokeStyle = `rgba(17,18,21,${0.55 * seam})`;
    g.lineWidth = Math.max(1 / k, px(s * 0.06));
    g.beginPath(); g.moveTo(sx - w, top); g.lineTo(sx, top - hh); g.lineTo(sx + w, top); g.lineTo(sx + w, sy); g.lineTo(sx, sy + hh); g.lineTo(sx - w, sy); g.closePath();
    g.moveTo(sx - w, top); g.lineTo(sx, top + hh); g.lineTo(sx + w, top); g.moveTo(sx, top + hh); g.lineTo(sx, sy + hh);
    g.stroke();
  }
}

/** The quick launch mark: about a second, never longer than this. */
export const SHORT = 1.0;

export interface IntroOpts { full: boolean; calm: boolean; font: string }

/** Length of each intro in seconds. */
export function introLength(o: IntroOpts): number {
  return o.full ? (o.calm ? 4.8 : 5.6) : o.calm ? 0.8 : SHORT;
}

/** Draw the intro at time t (seconds) into a W x H (CSS px) context. */
export function drawIntro(g: CanvasRenderingContext2D, t: number, W: number, H: number, o: IntroOpts): void {
  g.fillStyle = INK;
  g.fillRect(0, 0, W, H);
  // First launch always gets the full build; with Reduce motion the blocks
  // settle in place instead of falling.
  const full = o.full;
  const s = full ? Math.min(W, H) / 46 : Math.min(W, H) / 70; // voxel size
  const lift = full ? s * 2.6 * eout((t - 3.55) / 0.7) : 0; // logo rises to make room for the name
  const cx = W / 2, cy = H / 2 - lift;
  const P = (x: number, y: number, z: number) => [cx + (x - y) * 0.866 * s, cy + ((x + y) * 0.5 - z) * s] as const;
  const centre = (N - 1) / 2;

  if (!full) {
    // Every later launch: the sharp-edged logo B settles in about a second.
    // Each block drops a short way and lands (staggered from the ground up),
    // the teal cube drops in, a short hold, then the cover fades.
    const out = o.calm ? 1 - eout((t - 0.55) / 0.25) : 1 - eout((t - 0.78) / 0.22);
    const FALL = 0.22, SPREAD = 0.34, DROP = 2.2 * s;
    for (const v of RING) {
      const t0 = 0.02 + v.land * SPREAD;
      if (t < t0) continue;
      const u = clamp((t - t0) / FALL);
      const dy = o.calm ? 0 : -DROP * (1 - eout(u)) - (u >= 1 ? bounce(t - t0 - FALL, s * 0.18) : 0);
      const [x, y] = P(v.x - centre, v.y - centre, v.z - centre);
      g.globalAlpha = clamp(u * 2.5);
      cube(g, x, y + dy, s, FACE, 0);
    }
    const tt = 0.4; // teal cube drops in as the last blocks land
    if (t >= tt) {
      const u = clamp((t - tt) / 0.2);
      const dy = o.calm ? 0 : -4 * s * (1 - u * u) - (u >= 1 ? bounce(t - tt - 0.2, s * 0.35) : 0);
      g.globalAlpha = clamp(u * 3);
      drawTeal(g, P, s, dy, 0);
    }
    // Fade the finished mark as one piece (an Ink veil), so overlapping
    // blocks never show see-through seams.
    g.globalAlpha = 1 - out;
    g.fillStyle = INK;
    g.fillRect(0, 0, W, H);
    g.globalAlpha = 1;
    return;
  }

  // 1. Ring voxels pour in from the ground up (0.3s to 2.4s).
  const BUILD0 = 0.25, BUILD = 2.15, FALL = 0.42, DROP = 9 * s;
  const seam = 1 - eout((t - 3.4) / 0.55);
  const ct = 2.85, cl = 3.25; // teal cube: starts falling, lands
  const thud = o.calm ? 0 : -bounce(t - cl, s * 0.18); // the C gives a little as the cube lands
  for (const v of RING) {
    const tl = BUILD0 + v.land * BUILD + FALL; // landing time
    const t0 = tl - FALL;
    if (t < t0) continue;
    let dy: number, squash = 1;
    if (o.calm) dy = 0;
    else if (t < tl) { const u = (t - t0) / FALL; dy = -DROP * (1 - u * u); } // gravity
    else {
      dy = -bounce(t - tl, s * 0.55);
      const k = t - tl;
      if (k < 0.08) squash = 1 - 0.18 * Math.sin((k / 0.08) * Math.PI);
    }
    const a = clamp((t - t0) / (o.calm ? 0.3 : 0.12));
    const [x, y] = P(v.x - centre, v.y - centre, v.z - centre);
    g.globalAlpha = a;
    cube(g, x, y + dy + thud, s, FACE, seam, squash);
  }
  g.globalAlpha = 1;
  // 2. The teal cube falls from the top of the screen and bounces into place (2.3s to 3.0s).
  if (t >= ct) {
    const u = clamp((t - ct) / (cl - ct));
    if (o.calm) {
      g.globalAlpha = u;
      drawTeal(g, P, s, 0, seam);
      g.globalAlpha = 1;
    } else {
      const dy = t < cl ? -(cy + 4 * s) * (1 - u * u) : -bounce(t - cl, s * 1.6);
      drawTeal(g, P, s, dy, seam);
    }
  }
  // 3. Name and tagline.
  const word = "Cove";
  g.textBaseline = "alphabetic";
  g.font = `600 ${Math.round(s * 2.6)}px ${o.font}`;
  g.textAlign = "left";
  let wx = cx - g.measureText(word).width / 2;
  const base = H / 2 + N * 0.95 * s - lift + s * 2.4;
  for (let i = 0; i < word.length; i++) {
    const lt = eout((t - 3.75 - i * 0.07) / 0.45);
    if (lt > 0) { g.globalAlpha = lt; g.fillStyle = "#f2eee9"; g.fillText(word[i], wx, base + 14 * (1 - lt)); }
    wx += g.measureText(word[i]).width;
  }
  const ta = eout((t - 4.25) / 0.5);
  if (ta > 0) {
    g.globalAlpha = ta; g.textAlign = "center"; g.fillStyle = "#8b8b93";
    g.font = `400 ${Math.round(s * 1.05)}px ${o.font}`;
    g.fillText("Windows, made calm.", cx, base + s * 1.7);
  }
  g.globalAlpha = 1;
}

function drawTeal(g: CanvasRenderingContext2D, P: (x: number, y: number, z: number) => readonly [number, number], s: number, dy: number, seam: number) {
  // A 3x3x3 block of voxels sitting in the C's opening, drawn back to front.
  const shell: [number, number, number][] = [];
  for (let x = -1; x <= 1; x++) for (let y = -1; y <= 1; y++) for (let z = -1; z <= 1; z++)
    if (x === 1 || y === 1 || z === 1) shell.push([x, y, z]); // only the visible shell
  shell.sort((a, b) => a[0] + a[1] + a[2] - (b[0] + b[1] + b[2]) || a[2] - b[2]);
  for (const [x, y, z] of shell) {
    const [sx, sy] = P(x, y, z);
    cube(g, sx, sy + dy, s, TEAL, seam);
  }
}
