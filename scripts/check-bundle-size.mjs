// Performance gate: the UI that ships inside Cove stays small.
// Run after `vite build` (CI does): node scripts/check-bundle-size.mjs [distDir]
// Counts the code Vite emits (JS + CSS + HTML), not images or wallpapers.
// Raise a budget only on purpose, in the same change that needs it.
import { readdirSync, statSync, readFileSync } from "node:fs";
import { join, extname, relative } from "node:path";
import { gzipSync } from "node:zlib";

const BUDGET = {
  totalKB: 300,       // all JS + CSS + HTML, uncompressed (2026-11-05: 186 KB)
  totalGzipKB: 110,   // same, gzipped (2026-11-05: 64 KB)
  chunkKB: 96,        // any single JS or CSS file (2026-11-05 largest: Settings JS, 37 KB)
};

const dist = process.argv[2] ?? "ui/dist";
const files = [];
(function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) { if (name !== "wallpapers") walk(p); continue; }
    if ([".js", ".css", ".html"].includes(extname(name))) files.push(p);
  }
})(dist);
if (!files.length) { console.error(`No built UI in ${dist}. Run \`npx vite build\` first.`); process.exit(1); }

let total = 0, gz = 0;
const fails = [];
const rows = files.map((p) => {
  const buf = readFileSync(p);
  const kb = buf.length / 1024, gkb = gzipSync(buf).length / 1024;
  total += kb; gz += gkb;
  if (extname(p) !== ".html" && kb > BUDGET.chunkKB) fails.push(`${relative(dist, p)} is ${kb.toFixed(1)} KB (budget ${BUDGET.chunkKB} KB per file)`);
  return [relative(dist, p), kb, gkb];
}).sort((a, b) => b[1] - a[1]);

for (const [n, kb, gkb] of rows.slice(0, 8)) console.log(`${kb.toFixed(1).padStart(7)} KB  ${gkb.toFixed(1).padStart(6)} KB gz  ${n}`);
console.log(`UI total: ${total.toFixed(1)} KB (${gz.toFixed(1)} KB gzipped) across ${files.length} files`);
if (total > BUDGET.totalKB) fails.push(`UI total is ${total.toFixed(1)} KB (budget ${BUDGET.totalKB} KB)`);
if (gz > BUDGET.totalGzipKB) fails.push(`UI gzipped total is ${gz.toFixed(1)} KB (budget ${BUDGET.totalGzipKB} KB)`);
if (fails.length) { console.error("Bundle size budget exceeded:\n  " + fails.join("\n  ")); process.exit(1); }
console.log("Bundle size budget: OK");
