// The recipe panel of a generated spec and «Aplicar», in headless Chrome.
//   node app_generate.mjs --out DIR --port PORT [--dir PROJECT_DIR]
// The target is a scratch copy of the examples with specs/rock.toml and a fake Godot.
import fs from "node:fs";
import path from "node:path";
import { launchChrome } from "./cdp.mjs";
import { Driver } from "./driver.mjs";

const arg = n => { const i = process.argv.indexOf("--" + n); return i > 0 ? process.argv[i + 1] : null; };
const OUT = arg("out"), PORT = arg("port"), DIR = arg("dir");
if (!OUT || !PORT) { console.error("usage: app_generate.mjs --out DIR --port PORT [--dir PROJECT_DIR]"); process.exit(2); }

const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok: !!ok, detail });
  console.log(`  ${ok ? "ok  " : "FAIL"} generated/${name}${ok || !detail ? "" : "  -> " + detail}`);
};
const q = JSON.stringify;
const base = `http://127.0.0.1:${PORT}`;
const text = (d, sel) => d.page.eval(`document.querySelector(${q(sel)})?.innerText ?? ''`);
const count = (d, sel) => d.page.eval(`document.querySelectorAll(${q(sel)}).length`);
const value = (d, sel) => d.page.eval(`document.querySelector(${q(sel)})?.value ?? null`);
async function go(d, id) {
  await d.goto(`${base}/#/spec/${encodeURIComponent(id)}`);
  const ok = await d.waitFor(`() => document.querySelector('#main [data-view="spec"][data-ready]')`);
  await d.settle({ quiet: 250 });
  return ok;
}
async function fill(d, sel, v) {
  await d.click(`() => document.querySelector(${q(sel)})`);
  await d.page.eval(`document.querySelector(${q(sel)}).select()`);
  await d.type(String(v));
}

fs.mkdirSync(OUT, { recursive: true });
const chrome = await launchChrome(path.join(OUT, "chrome-profile-generate"));
process.on("exit", () => chrome.kill());
const d = await Driver.open(chrome.port, "generated");
try {
  const posts = [];
  d.page.on("Network.requestWillBeSent", e => { if (e.request.method === "POST") posts.push(e.request.url); });

  check("rock: the spec view opens", await go(d, "rock"));
  check("rock: no parts table", (await count(d, "table.tbl tr[data-part]")) === 0);
  const groups = await d.page.eval(`[...document.querySelectorAll('#genRoot fieldset[data-op] legend')].map(l => l.textContent.trim())`);
  check("rock: one group per op with its Spanish label", groups.length === 3 && groups.every(g => g.length > 0 && !["sphere", "noise", "facet"].includes(g)), JSON.stringify(groups));
  check("rock: radius range has min and max inputs", (await count(d, "#g0_radius_m_lo")) === 1 && (await count(d, "#g0_radius_m_hi")) === 1 &&
    (await value(d, "#g0_radius_m_lo")) === "2" && (await value(d, "#g0_radius_m_hi")) === "5");
  check("rock: the seed input shows 7", (await value(d, "#genSeed")) === "7");
  check("rock: the Aplicar button", (await text(d, "#genRoot button[data-apply]")).trim() === "Aplicar");

  await fill(d, "#genSeed", 9);
  await fill(d, "#g1_amplitude_hi", 0.4);
  check("rock: the edits are in the inputs before applying", (await value(d, "#genSeed")) === "9" && (await value(d, "#g1_amplitude_hi")) === "0.4",
    `${await value(d, "#genSeed")} ${await value(d, "#g1_amplitude_hi")}`);
  await d.click(`() => document.querySelector('#genRoot button[data-apply]')`);
  const done = await d.waitFor(`() => [...document.querySelectorAll('.toast')].some(t => /Asset generado/.test(t.textContent))`, 30000);
  check("apply: the toast «Asset generado» appears", done, (await text(d, ".toast")).slice(0, 120));
  await d.waitFor(`() => document.querySelector('#g3d canvas')`, 15000);
  await d.settle({ quiet: 300 });

  check("apply: the page posted /api/generate", posts.some(u => u.endsWith("/api/generate")), JSON.stringify(posts));
  const cat = await (await fetch(`${base}/api/catalog`, { headers: { "X-Kiln-Fresh": "1" } })).json();
  const g = cat.specs.find(s => s.asset.id === "rock")?.generator;
  const noise = g?.ops.find(o => o.op === "noise");
  check("apply: the catalog has seed 9 and the new amplitude", g?.seed === 9 && JSON.stringify(noise?.amplitude) === "[0.1,0.4]", JSON.stringify(g));
  if (DIR) {
    const toml = fs.readFileSync(path.join(DIR, "specs/rock.toml"), "utf8");
    check("apply: specs/rock.toml on disk has seed = 9 and the new amplitude", /^seed = 9$/m.test(toml) && /amplitude = \[\s*0\.1,\s*0\.4\s*\]/.test(toml), toml.slice(0, 300));
  }
  check("apply: the 3D view has a canvas", (await count(d, "#g3d canvas")) === 1);
  const r = await fetch(`${base}/files/kiln/cache/generated/rock/9.glb`);
  check("apply: the generated GLB is served", r.status === 200, `status ${r.status}`);

  check("hoist: still shows its parts table and no recipe panel", await go(d, "hoist") && (await count(d, "tr[data-part]")) > 0 && (await count(d, "#genRoot")) === 0);
  check("no JS errors", d.errors.length === 0, d.errors.join(" | "));
} catch (e) {
  check("script ran to the end", false, e.stack || String(e));
} finally {
  await d.close().catch(() => {});
  chrome.kill();
}
const failed = results.filter(r => !r.ok);
console.log(`\n${results.length - failed.length} generated checks passed, ${failed.length} failed`);
process.exit(failed.length ? 1 : 0);
