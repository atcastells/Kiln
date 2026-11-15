// Regression walkthrough and measurements of the Kiln web UI, in headless Chrome.
//   node walkthrough.mjs --out DIR --target name=PORT ... [--update-baseline]
// Targets: examples, content, synthetic (measured too), wizard (folder without kiln.toml).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchChrome, sleep } from "./cdp.mjs";
import { Driver, find } from "./driver.mjs";
import { S } from "./selectors.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const arg = n => { const i = process.argv.indexOf(n); return i < 0 ? null : process.argv[i + 1]; };
const OUT = arg("--out"), UPDATE = process.argv.includes("--update-baseline");
const BASELINE = arg("--baseline") || path.join(here, "baseline.json");
const RUNS = Number(arg("--runs") || 3);
const targets = Object.fromEntries(process.argv.flatMap((a, i) => a === "--target" ? [process.argv[i + 1].split("=")] : []));
if (!OUT || !Object.keys(targets).length) { console.error("usage: walkthrough.mjs --out DIR --target name=PORT [...]"); process.exit(2); }
fs.mkdirSync(path.join(OUT, "screenshots"), { recursive: true });

const results = [], notes = [];
const check = (target, name, ok, detail = "") => {
  results.push({ id: `${target}/${name}`, ok: !!ok, detail });
  console.log(`  ${ok ? "ok  " : "FAIL"} ${target}/${name}${ok || !detail ? "" : "  -> " + detail}`);
};
const note = (target, text) => { notes.push(`${target}: ${text}`); console.log(`  note ${target}: ${text}`); };
const api = async (port, p) => (await fetch(`http://127.0.0.1:${port}${p}`)).json();
const median = a => { const s = [...a].sort((x, y) => x - y); return s.length % 2 ? s[s.length >> 1] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2; };

async function openProject(chrome, name, port) {
  const d = await Driver.open(chrome.port, name);
  await d.goto(`http://127.0.0.1:${port}/`);
  return d;
}
const shot = (d, name, view) => d.page.screenshot(path.join(OUT, "screenshots", `${name}-${view}.png`), fs);

// ---------------------------------------------------------------- walkthrough
async function walk(chrome, name, port) {
  console.log(`\n== ${name} (port ${port})`);
  const d = await openProject(chrome, name, port);
  try {
    if (name === "wizard") return await walkWizard(d, name);
    const cat = await api(port, "/api/catalog"), ctx = await api(port, "/api/context");
    const ready = await d.waitFor(S.meaningful);
    await d.settle({ quiet: 400 });
    check(name, "load: project drawn", ready, "the page never showed items and a filled side panel");
    check(name, "load: no JS errors", d.errors.length === 0, d.errors.join(" | "));
    check(name, "load: project name shown", (await d.page.eval(`document.querySelector(${JSON.stringify(S.projectTag)})?.textContent`)) === cat.project);
    await shot(d, name, "inicio");

    for (const [view, label] of Object.entries(S.views)) {
      const id = `view ${label}`;
      const shown = await d.buttonVisible(label);
      const expected = view !== "content" || cat.content.length > 0;
      check(name, `${id}: tab ${expected ? "present" : "absent (no content sets)"}`, shown === expected);
      if (!shown) continue;
      const mark = d.errors.length;
      await d.clickAndSettle(find.button(label));
      await shot(d, name, view);
      check(name, `${id}: no JS errors`, d.errors.length === mark, d.errors.slice(mark).join(" | "));
      await VIEW[view](d, name, port, cat, ctx);
    }
    await specFlow(d, name, cat);
    await reloadFlow(d, name);
  } catch (e) {
    check(name, "walkthrough ran to the end", false, e.stack?.split("\n").slice(0, 3).join(" "));
  } finally { await d.close(); }
}

const matches = (q, text) => String(text).toLowerCase().includes(q);

async function searchNarrows(d, name, view, q, expectDim) {
  const mark = d.errors.length;
  await d.focusSearch(); await d.type(q); await d.settle();
  const items = await d.items();
  const dim = items.filter(i => i.dim).length;
  check(name, `view ${view}: search "${q}" narrows (${expectDim} dimmed)`, dim === expectDim && expectDim > 0, `dimmed ${dim} of ${items.length}`);
  check(name, `view ${view}: search no JS errors`, d.errors.length === mark, d.errors.slice(mark).join(" | "));
  return items;
}

const VIEW = {
  async context(d, name, port, cat, ctx) {
    const items = await d.items();
    const layers = ctx.layers.map(l => l.name);
    check(name, "view Contexto: shows the game node", items.some(i => i.title === cat.project && i.kind.startsWith("Juego")));
    check(name, "view Contexto: one node per layer", items.filter(i => i.kind.startsWith("Capa")).length >= layers.length, `${items.filter(i => i.kind.startsWith("Capa")).length} < ${layers.length}`);
    check(name, "view Contexto: shows each claim", items.filter(i => /^(Afirmaci|Fijada)/.test(i.kind)).length === ctx.context.claims.length);
    check(name, "view Contexto: chat panel visible", await d.visible(S.chat));
    check(name, "view Contexto: side panel filled", (await d.side()).includes(cat.project));
    // search: the claims that do not contain the word are dimmed
    if (ctx.context.claims.length > 1) {
      const word = ctx.context.claims[0].text.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(w => w.length > 4).sort((a, b) => b.length - a.length)[0];
      const dimmed = ctx.context.claims.filter(c => !matches(word, c.text)).length;
      if (word && dimmed > 0) await searchNarrows(d, name, "Contexto", word, dimmed); else note(name, "Contexto search: no discriminating word among the claims");
      await d.clearSearch(); await d.settle();
    } else note(name, "Contexto search skipped: the project has fewer than 2 claims");
    if (layers.length) {
      const title = layers[0], before = await d.side();
      await d.clickAndSettle(find.item(title));
      const after = await d.side();
      check(name, "view Contexto: selecting a layer fills the side panel", after !== before && after.includes(title), after.slice(0, 80));
    }
  },

  async tree(d, name, port, cat) {
    if (!cat.tree) return note(name, "Árbol: project without tree");
    const T = cat.tree.nodes, root = T[cat.tree.root], items = await d.items();
    check(name, "view Árbol: root node shown", items.some(i => i.title === root.name));
    const kids = root.children.map(k => T[k].name);
    check(name, "view Árbol: root's children shown", kids.every(k => items.some(i => i.title === k)), `children ${kids.length}`);
    check(name, "view Árbol: side panel filled", (await d.side()).includes(root.name));
    if (kids.length) {
      const before = await d.side();
      await d.clickAndSettle(find.item(kids[0]));
      const after = await d.side();
      check(name, "view Árbol: selecting a node fills the side panel", after !== before && after.includes(kids[0]), after.slice(0, 80));
    }
    // The search box of this UI only offers assets that have a spec.
    const spec = cat.specs[0];
    await d.focusSearch();
    if (spec) {
      await d.type(spec.asset.name.toLowerCase().slice(0, 4)); await d.settle();
      const hits = await d.hits();
      check(name, "view Árbol: search lists the matching asset", hits.some(h => h.includes(spec.asset.name)), JSON.stringify(hits));
      await d.clearSearch(); await d.type("zzzqq"); await d.settle();
      check(name, "view Árbol: nonsense search lists nothing", (await d.hits()).length === 0);
    } else {
      await d.type("kit0"); await d.settle();
      check(name, "view Árbol: search with no specs lists nothing and does not break", (await d.hits()).length === 0);
      note(name, "Árbol search only matches asset specs; this project has none, so nothing narrows");
    }
    await d.clearSearch(); await d.settle();
  },

  async content(d, name, port, cat) {
    const heads = await d.contentHeadings();
    check(name, "view Contenido: container visible", await d.visible(S.content));
    for (const set of cat.content) {
      check(name, `view Contenido: lists set "${set.name}"`, heads.includes(set.name), JSON.stringify(heads));
    }
    const rows = await d.contentRows(), want = cat.content.reduce((n, s) => n + s.elements.length, 0);
    check(name, `view Contenido: one row per element (${want})`, rows === want, `${rows} rows`);
    check(name, "view Contenido: side panel filled", (await d.side()).length > 0);
    // Search: this view ignores it (only the assets dropdown reacts). Recorded, not asserted.
    await d.focusSearch(); await d.type(cat.content[0].elements[0].id.slice(0, 6)); await d.settle();
    const after = await d.contentRows();
    note(name, `Contenido: search ${after === rows ? "does NOT narrow the table" : "narrows the table"} (${rows} -> ${after} rows)`);
    check(name, "view Contenido: typing in search keeps the table", after > 0 && d.errors.length === 0, d.errors.join(" | "));
    await d.clearSearch(); await d.settle();
    note(name, "Contenido: rows are not selectable; the side panel only shows the summary");
  },

  async catalog(d, name, port, cat) {
    let items = await d.items();
    const kits = items.filter(i => i.kind.startsWith("Kit")), mats = items.filter(i => i.kind.startsWith("Material"));
    check(name, `view Catálogo: lists every kit (${cat.kits.length})`, cat.kits.every(k => kits.some(i => i.title === k.name)) && kits.length === cat.kits.length, `${kits.length} shown`);
    check(name, `view Catálogo: lists materials (${Math.min(80, cat.materials.length)}${cat.materials.length > 80 ? ", cut at 80" : ""})`, mats.length === Math.min(80, cat.materials.length), `${mats.length} shown`);
    if (cat.materials.length > 80) note(name, `Catálogo shows ${mats.length} of ${cat.materials.length} materials, with no sign that it cut the list`);
    check(name, "view Catálogo: side panel filled", (await d.side()).includes(cat.project));
    // Search: kits and materials that do not match are dimmed or dropped.
    const last = cat.kits.at(-1).name.toLowerCase();
    const keep = cat.kits.filter(k => matches(last, k.name) || cat.pieces.some(p => p.kit === k.name && matches(last, p.key))).length;
    if (cat.kits.length > 1) {
      const found = await searchNarrows(d, name, "Catálogo", last, cat.kits.length - keep);
      const matsLeft = found.filter(i => i.kind.startsWith("Material")).length;
      check(name, "view Catálogo: search also filters materials", matsLeft === cat.materials.filter(m => matches(last, m.key) || matches(last, m.name)).length, `${matsLeft} left`);
      await d.clearSearch(); await d.settle();
      items = await d.items();
      check(name, "view Catálogo: clearing the search restores everything", items.every(i => !i.dim));
    }
    // Select a kit: it opens its pieces and fills the side panel.
    const kit = cat.kits[0], before = await d.side();
    await d.clickAndSettle(find.item(kit.name));
    const after = await d.side();
    check(name, "view Catálogo: selecting a kit fills the side panel", after !== before && after.includes(kit.name), after.slice(0, 80));
    items = await d.items();
    const pieces = items.filter(i => i.kind.startsWith("Pieza"));
    check(name, `view Catálogo: an open kit lists its pieces (${kit.pieces})`, pieces.length === kit.pieces, `${pieces.length} shown`);
    if (pieces.length) {
      const mark = d.errors.length, p = pieces[0], prev = await d.side();
      await d.clickAndSettle(find.item(p.title));
      await sleep(1200); await d.settle(); // the 3D preview loads on demand
      const side = await d.side();
      check(name, "view Catálogo: selecting a piece fills the side panel", side !== prev && side.includes(p.title), side.slice(0, 80));
      check(name, "view Catálogo: selecting a piece raises no JS error (3D preview included)", d.errors.length === mark, d.errors.slice(mark).join(" | "));
      const has3d = await d.page.eval("!!document.querySelector('#v3d canvas')");
      note(name, `piece preview: ${has3d ? "3D canvas drawn" : "no 3D canvas (" + ((await d.page.eval("document.querySelector('#v3d')?.innerText ?? 'no preview box'"))).slice(0, 60) + ")"}`);
      await shot(d, name, "catalogo-pieza");
    }
    const m = (await d.items()).find(i => i.kind.startsWith("Material"));
    if (m) {
      const prev = await d.side();
      await d.clickAndSettle(find.item(m.title));
      const side = await d.side();
      check(name, "view Catálogo: selecting a material fills the side panel", side !== prev && side.includes(m.title), side.slice(0, 80));
    }
  },
};

// A project with specs: search offers the asset, its sheet opens, a node of it fills the panel, Volver returns.
async function specFlow(d, name, cat) {
  const spec = cat.specs[0];
  if (!spec) return note(name, "spec/build sub-view skipped: project without specs");
  const mark = d.errors.length;
  await d.clickAndSettle(find.button(S.views.tree));
  await d.focusSearch(); await d.type(spec.asset.name.toLowerCase().slice(0, 4)); await d.settle();
  await d.clickAndSettle(find.hit());
  const items = await d.items();
  check(name, "spec sheet: opens from the search", items.some(i => i.kind.startsWith("Petición")) && await d.buttonVisible(S.backLabel), items.map(i => i.kind).join(","));
  check(name, "spec sheet: side panel filled", (await d.side()).includes(spec.asset.name));
  await shot(d, name, "ficha");
  const build = items.find(i => i.kind.startsWith("Montaje"));
  if (build) {
    const prev = await d.side();
    await d.clickAndSettle(find.itemKind("Montaje"));
    check(name, "build node: fills the side panel", (await d.side()) !== prev);
    await shot(d, name, "construccion");
  }
  await d.clickAndSettle(find.button(S.backLabel));
  check(name, "spec sheet: Volver returns to the tree", !(await d.buttonVisible(S.backLabel)) && (await d.items()).some(i => i.title === cat.tree.nodes[cat.tree.root].name));
  check(name, "spec sheet: no JS errors", d.errors.length === mark, d.errors.slice(mark).join(" | "));
  await d.clearSearch(); await d.settle();
  note(name, "review view and the 3D of a built asset need Blender output: not covered");
}

async function reloadFlow(d, name) {
  const mark = d.errors.length, before = (await d.items()).length;
  await d.clickAndSettle(find.button(S.reloadLabel), {});
  const after = (await d.items()).length;
  check(name, "Releer el proyecto: responds, same items, no errors", after === before && d.errors.length === mark, `${before} -> ${after} ${d.errors.slice(mark).join(" | ")}`);
}

async function walkWizard(d, name) {
  const ok = await d.waitFor(`() => __vis(document.querySelector(${JSON.stringify(S.wizard)}))`);
  await d.settle({ quiet: 400 });
  check(name, "start wizard: opens on a folder without kiln.toml", ok);
  check(name, "start wizard: has a text field and a primary button", await d.page.eval(`!!document.querySelector(${JSON.stringify(S.wizard + " input")}) && !!document.querySelector(${JSON.stringify(S.wizard + " button")})`));
  check(name, "start wizard: chat panel visible", await d.visible(S.chat));
  check(name, "start wizard: no JS errors", d.errors.length === 0, d.errors.join(" | "));
  await shot(d, name, "asistente");
  note(name, "wizard steps beyond the first are not driven (they scaffold files and call the agent)");
}

// ----------------------------------------------------------------- measurements
async function measureOnce(chrome, port, kit) {
  const d = await openProject(chrome, "measure", port), m = {};
  try {
    await d.waitFor(`() => __k.fmp != null`, 60000);
    await d.settle({ quiet: 400 });
    const t = await d.page.eval("({ fmp: __k.fmp, paint: __k.fmpPaint, fcp: performance.getEntriesByName('first-contentful-paint')[0]?.startTime })");
    Object.assign(m, { fmp_dom_ms: t.fmp, fmp_paint_ms: t.paint, fcp_ms: t.fcp });
    m.heap_default_mb = await d.heapMB();
    m.dom_default = await d.domCount();
    for (const [view, label] of Object.entries(S.views)) {
      if (!(await d.buttonVisible(label))) continue;
      const s = await d.clickAndSettle(find.button(label));
      m[`switch_${view}_ms`] = s.ms; m[`dom_${view}`] = await d.domCount();
    }
    // typing in Catálogo, kits closed: the last key of "kit03"
    await d.focusSearch(); await d.type(kit.slice(0, -1)); await d.settle();
    await d.arm(); await d.page.key(kit.at(-1)); const k1 = await d.settle();
    m.keystroke_ms = k1.ms;
    // open the kit (many pieces), then type again with it open
    const k2 = await d.clickAndSettle(find.item(kit));
    m.open_kit_ms = k2.ms; m.long_open_kit_ms = k2.long; m.dom_catalog_kit_open = await d.domCount();
    await d.focusSearch(); await d.arm(); await d.page.key("Backspace"); const k3 = await d.settle();
    m.keystroke_open_kit_ms = k3.ms;
    m.heap_after_views_mb = await d.heapMB();
    m.js_errors = d.errors.length;
  } finally { await d.close(); }
  return m;
}

async function measure(chrome, port) {
  console.log(`\n== measurements on the synthetic project (${RUNS} runs, median)`);
  const cat = await api(port, "/api/catalog");
  const kit = cat.kits[Math.min(3, cat.kits.length - 1)].name;
  const runs = [];
  for (let i = 0; i < RUNS; i++) { runs.push(await measureOnce(chrome, port, kit)); console.log(`  run ${i + 1} done`); }
  const api_ms = [], bytes = [];
  for (let i = 0; i < RUNS; i++) { const t = performance.now(); const r = await fetch(`http://127.0.0.1:${port}/api/catalog`); const b = (await r.arrayBuffer()).byteLength; api_ms.push(performance.now() - t); bytes.push(b); }
  const out = { api_catalog_ms: median(api_ms), api_catalog_kb: median(bytes) / 1024, pieces: cat.pieces.length, findings: cat.findings.length };
  for (const k of Object.keys(runs[0])) out[k] = median(runs.map(r => r[k]).filter(v => v != null));
  return { median: out, runs };
}

// ------------------------------------------------------------------------ main
const chrome = await launchChrome(path.join(OUT, "chrome-profile"));
process.on("SIGTERM", () => { chrome.kill(); process.exit(143); });
process.on("SIGINT", () => { chrome.kill(); process.exit(130); });
console.log(`Chrome: ${chrome.version}`);
let perf = null;
try {
  for (const [name, port] of Object.entries(targets)) await walk(chrome, name, port);
  if (targets.synthetic) perf = await measure(chrome, targets.synthetic);
} finally { chrome.kill(); }

// ---- report and baseline
const failed = results.filter(r => !r.ok);
const base = !UPDATE && fs.existsSync(BASELINE) ? JSON.parse(fs.readFileSync(BASELINE, "utf8")) : null;
const regressions = [];
if (!UPDATE) {
  if (!base) regressions.push(`no baseline at ${path.relative(process.cwd(), BASELINE)}; run with --update-baseline`);
  else for (const id of Object.keys(base.checks)) if (base.checks[id] && !results.some(r => r.id === id && r.ok)) regressions.push(`check from the baseline is missing or red: ${id}`);
}
const fmt = v => v == null ? "" : Math.abs(v) >= 100 ? v.toFixed(0) : v.toFixed(1);
if (perf) {
  const rows = Object.entries(perf.median).map(([k, v]) => {
    const b = base?.metrics?.[k];
    const delta = b == null ? "" : (v - b >= 0 ? "+" : "") + fmt(v - b) + (b ? ` (${((v - b) / b * 100).toFixed(0)}%)` : "");
    return [k, fmt(v), b == null ? "" : fmt(b), delta];
  });
  const w = [0, 1, 2, 3].map(i => Math.max(...["metric,median,baseline,delta".split(",")[i].length, ...rows.map(r => r[i].length)]));
  const line = r => r.map((c, i) => i ? c.padStart(w[i]) : c.padEnd(w[i])).join("  ");
  console.log("\n" + line(["metric", "median", "baseline", "delta"]) + "\n" + w.map(n => "-".repeat(n)).join("  ") + "\n" + rows.map(line).join("\n"));
  fs.writeFileSync(path.join(OUT, "metrics.json"), JSON.stringify(perf, null, 2));
}
fs.writeFileSync(path.join(OUT, "walkthrough.json"), JSON.stringify({ results, notes }, null, 2));
if (UPDATE && failed.length) console.log("\nbaseline NOT written: fix the failing checks first");
else if (UPDATE) {
  const checks = Object.fromEntries(results.map(r => [r.id, r.ok]));
  fs.writeFileSync(BASELINE, JSON.stringify({ chrome: chrome.version, checks, metrics: perf?.median ?? {}, notes }, null, 2) + "\n");
  console.log(`\nbaseline written: ${BASELINE}`);
}
console.log(`\n${results.length - failed.length} checks passed, ${failed.length} failed, ${notes.length} notes`);
for (const f of failed) console.log(`FAIL ${f.id}: ${f.detail}`);
for (const r of regressions) console.log(`REGRESSION ${r}`);
process.exit(failed.length || regressions.length ? 1 : 0);
