#!/usr/bin/env node
// Re-verify the wallpaper gallery (ui/public/wallpapers/manifest.json).
// Run by hand before a release: node scripts/check-wallpapers.mjs [--deep]
//
// For every Wikimedia Commons / NASA photo it asks the Commons API
// (imageinfo + extmetadata) that the file still exists, that its licence is
// still free (CC0, CC BY, CC BY-SA or public domain) and matches the manifest,
// that it is at least 2560 px wide, and that the file URL answers.
// For Cove art on the wallpapers-v1 release it checks the URL answers with the
// right size; --deep downloads each one and checks its sha256.
// Every entry also needs a thumbnail in ui/public/wallpapers/.
// No API key, no CI: polite, one request at a time, with a descriptive User-Agent.

import { readFileSync, existsSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const UA = `Cove/${pkg.version} (https://github.com/zernivo/Cove) check-wallpapers`;
const API = "https://commons.wikimedia.org/w/api.php";
const FREE = /^(CC0( 1\.0)?|CC BY(-SA)? [0-9.]+|Public domain)$/i;
const deep = process.argv.includes("--deep");
const manifest = JSON.parse(readFileSync(join(root, "ui/public/wallpapers/manifest.json"), "utf8"));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const strip = (s) => (s ?? "").replace(/<[^>]+>/g, "").trim();
const problems = [];
const bad = (id, msg) => { problems.push(`${id}: ${msg}`); console.log(`  ✗ ${id}: ${msg}`); };

async function get(url, opts = {}) {
  for (let attempt = 0; attempt < 4; attempt++) {
    const r = await fetch(url, { ...opts, headers: { "User-Agent": UA, ...(opts.headers ?? {}) }, redirect: "follow" });
    if (r.status !== 429) return r;
    await sleep(5000 * (attempt + 1));
  }
  throw new Error("rate-limited (429)");
}

const cats = new Set(manifest.categories.map((c) => c.id));
const ids = new Set();
for (const it of manifest.items) {
  if (ids.has(it.id)) bad(it.id, "duplicate id");
  ids.add(it.id);
  if (!/^[a-z0-9-]+$/.test(it.id)) bad(it.id, "id must be lowercase words and hyphens");
  if (!cats.has(it.category)) bad(it.id, `unknown category ${it.category}`);
  if (!["dark", "light", "both"].includes(it.mode)) bad(it.id, `mode ${it.mode}`);
  for (const m of it.mode === "both" ? ["dark", "light"] : [it.mode]) {
    const t = join(root, "ui/public/wallpapers", it.thumb.replace("{mode}", m));
    if (!existsSync(t)) bad(it.id, `missing thumbnail ${it.thumb.replace("{mode}", m)}`);
  }
}

// Commons photos, 25 titles per API call.
const photos = manifest.items.filter((i) => i.commons);
console.log(`Checking ${photos.length} Commons/NASA photos…`);
for (let i = 0; i < photos.length; i += 25) {
  const batch = photos.slice(i, i + 25);
  const q = new URLSearchParams({
    action: "query", format: "json", formatversion: "2", prop: "imageinfo",
    iiprop: "url|size|extmetadata|mime", iiurlwidth: "3840", titles: batch.map((b) => b.commons).join("|"),
  });
  const data = await (await get(`${API}?${q}`)).json();
  const norm = new Map((data.query.normalized ?? []).map((n) => [n.from, n.to]));
  const pages = new Map(data.query.pages.map((p) => [p.title, p]));
  for (const it of batch) {
    const p = pages.get(norm.get(it.commons) ?? it.commons);
    if (!p || p.missing || !p.imageinfo) { bad(it.id, "file is gone from Commons"); continue; }
    const ii = p.imageinfo[0];
    const em = ii.extmetadata ?? {};
    let lic = strip(em.LicenseShortName?.value);
    if (lic === "CC0") lic = "CC0 1.0";
    if (!FREE.test(lic)) bad(it.id, `licence is now "${lic}"`);
    else if (lic.toLowerCase() !== it.license.toLowerCase()) bad(it.id, `licence changed: manifest "${it.license}", Commons "${lic}"`);
    if (ii.width < 2560) bad(it.id, `only ${ii.width}px wide`);
    if (ii.mime !== "image/jpeg") bad(it.id, `not a JPEG (${ii.mime})`);
    const want = (ii.width > 3840 ? ii.thumburl : ii.url).split("?")[0];
    if (want !== it.url) bad(it.id, `file URL changed to ${want}`);
    if (ii.descriptionurl !== it.sourceUrl) bad(it.id, `source page moved to ${ii.descriptionurl}`);
    if (!problems.some((x) => x.startsWith(it.id + ":"))) console.log(`  ✓ ${it.id} (${lic}, ${ii.width}px)`);
  }
  await sleep(1000);
}

// File URLs answer (HEAD; one at a time).
console.log("Checking file URLs…");
for (const it of manifest.items.filter((i) => i.url)) {
  try {
    const r = await get(it.url, { method: deep && it.sha256 ? "GET" : "HEAD" });
    if (!r.ok) { bad(it.id, `${it.url} answered ${r.status}`); continue; }
    const len = Number(r.headers.get("content-length") ?? 0);
    if (it.bytes && len && len !== it.bytes) bad(it.id, `size ${len}, manifest says ${it.bytes}`);
    if (deep && it.sha256) {
      const sum = createHash("sha256").update(Buffer.from(await r.arrayBuffer())).digest("hex");
      if (sum !== it.sha256) bad(it.id, "sha256 mismatch");
    }
  } catch (e) {
    bad(it.id, String(e));
  }
  await sleep(it.commons ? 1200 : 200);
}

const byCat = {};
for (const it of manifest.items) byCat[it.category] = (byCat[it.category] ?? 0) + 1;
console.log("Per category:", byCat);
if (problems.length) {
  console.log(`\n${problems.length} problem(s):\n` + problems.join("\n"));
  process.exit(1);
}
console.log("\nAll wallpapers check out.");
