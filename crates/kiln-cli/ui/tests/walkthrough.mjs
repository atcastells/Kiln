// Regression walkthrough and measurements of the Kiln web UI, in headless Chrome.
//   node walkthrough.mjs --out DIR --target name=PORT ... [--update-baseline]
// Targets: examples, content, synthetic (measured too), wizard (folder without kiln.toml).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchChrome, sleep } from "./cdp.mjs";
import { Driver, find } from "./driver.mjs";
import { S } from "./selectors.mjs";
import { walkAppViews, measureAppViews } from "./app_views.mjs";
import { walkNewViews, walkAppWizard } from "./app_views2.mjs";
import { walkAppA11y, walkLongNames } from "./app_a11y.mjs";

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
  await d.goto(`http://127.0.0.1:${port}/classic`); // the previous UI, kept one more release
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

// The new UI at / (and /app, kept so old links work): its module graph loads from the binary and it shows the project name.
const APP_PROBE = `(() => {
  const p = window.__app = { ready: null, paint: null };
  const look = () => { if (p.ready == null && document.querySelector('#main [data-view="home"][data-ready]')) { p.ready = performance.now(); requestAnimationFrame(() => requestAnimationFrame(() => { p.paint = performance.now(); })); } };
  new MutationObserver(look).observe(document, { subtree: true, childList: true, attributes: true });
})()`;
const SHOTS = process.env.KILN_APP_SHOTS || null;
const appShot = async (d, file) => {
  await d.page.screenshot(path.join(OUT, "screenshots", file), fs);
  if (SHOTS) { fs.mkdirSync(SHOTS, { recursive: true }); await d.page.screenshot(path.join(SHOTS, file), fs); }
};
const plural = n => new Intl.NumberFormat("es", { useGrouping: "always" }).format(n);

// The DOM part of the virtual list, the toasts and the dialog, driven inside the page.
const COMPONENTS = `(async () => {
  const out = {};
  const { createList } = await import('/ui/app/ui/list.js');
  const { createToasts } = await import('/ui/app/ui/feedback.js');
  const { askText } = await import('/ui/app/ui/dialog.js');
  const host = document.createElement('div'); host.style.cssText = 'position:fixed;left:0;top:0;width:700px;height:400px;z-index:500;background:var(--bg)'; document.body.appendChild(host);
  const rows = Array.from({ length: 5000 }, (_, i) => ({ k: 'piece' + String(i).padStart(4, '0'), n: (i * 7919) % 5000 }));
  let picked = null;
  const list = createList(host, { columns: [{ key: 'k', label: 'Nombre', mono: true }, { key: 'n', label: 'Triángulos', align: 'r' }], rows, rowId: r => r.k, label: 'prueba', onSelect: r => { picked = r.k; } });
  const grid = host.querySelector('.vgrid'), body = host.querySelector('.vb');
  const nodes = () => host.querySelectorAll('.vrow').length;
  out.rows0 = nodes(); out.count0 = host.querySelector('.vcount').textContent;
  body.scrollTop = 34 * 2500; body.dispatchEvent(new Event('scroll')); out.rowsScrolled = nodes();
  out.firstAfterScroll = host.querySelector('.vrow')?.textContent;
  host.querySelector('.hbtn[data-key=n]').click();
  out.sortAria = host.querySelector('[role=columnheader][data-key=n]').getAttribute('aria-sort');
  out.firstSorted = host.querySelector('.vrow .vc.r')?.textContent;
  list.setFilter(r => r.k.endsWith('99')); out.filtered = list.count(); out.countFiltered = host.querySelector('.vcount').textContent; out.rowsFiltered = nodes();
  list.setFilter(null);
  grid.focus();
  const key = k => grid.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));
  key('ArrowDown'); key('ArrowDown'); out.active = grid.getAttribute('aria-activedescendant');
  key('Enter'); out.picked = picked;
  key('End'); out.endRowRendered = !!host.querySelector('.vrow:last-child') && host.querySelector('.vrow.act') !== null; out.rowsAtEnd = nodes();
  key('Home'); out.afterHome = grid.getAttribute('aria-activedescendant');
  list.destroy(); host.remove();

  const wrap = document.getElementById('toastwrap'); wrap.querySelectorAll('.toast').forEach(t => t.remove());
  const toasts = createToasts(wrap, { setTimer: (f) => setTimeout(f, 60) });
  toasts.error('Falló', 'detalle'); toasts.ok('Hecho');
  out.alertToast = wrap.querySelectorAll('[data-region=alert] .toast.error').length;
  out.statusToast = wrap.querySelectorAll('[data-region=status] .toast.ok').length;
  await new Promise(r => setTimeout(r, 250));
  out.errorStays = wrap.querySelectorAll('[data-region=alert] .toast').length;
  out.okGone = wrap.querySelectorAll('[data-region=status] .toast').length;
  wrap.querySelector('[data-toast-close]').click(); out.errorClosed = wrap.querySelectorAll('.toast').length;
  out.regions = [wrap.querySelector('[data-region=status]').getAttribute('aria-live'), wrap.querySelector('[data-region=alert]').getAttribute('role')];

  const opener = document.createElement('button'); opener.textContent = 'abrir'; document.body.appendChild(opener); opener.focus();
  window.__dlg = askText(document, { title: 'Nombre', label: 'Nombre', validate: v => v ? '' : 'Escribe un nombre.' }).then(v => { window.__dlgResult = v; });
  out.dialogOpen = !!document.querySelector('dialog[open]'); out.dialogFocus = document.activeElement?.tagName;
  return out;
})()`;

async function walkComponents(d, name) {
  const mark = d.errors.length;
  const r = await d.page.eval(COMPONENTS);
  check(name, "list: 5,000 rows render fewer than 80 row nodes", r.rows0 > 0 && r.rows0 < 80 && r.rowsScrolled < 80 && r.rowsAtEnd < 80, JSON.stringify({ a: r.rows0, b: r.rowsScrolled, c: r.rowsAtEnd }));
  check(name, "list: scrolling renders the window at the new position", r.firstAfterScroll?.startsWith("piece2494") || r.firstAfterScroll?.startsWith("piece249"), r.firstAfterScroll);
  check(name, "list: counter shows the total", /5\D?000/.test(r.count0), r.count0);
  check(name, "list: header click sorts and sets aria-sort", r.sortAria === "ascending" && r.firstSorted === "0", `${r.sortAria} ${r.firstSorted}`);
  check(name, "list: filter narrows rows and counter", r.filtered === 50 && /50 de/.test(r.countFiltered) && r.rowsFiltered > 0 && r.rowsFiltered < 50, JSON.stringify([r.filtered, r.countFiltered, r.rowsFiltered]));
  check(name, "list: arrows move the active row, Enter selects, Home/End jump", /-r2$/.test(r.active) && r.picked && r.endRowRendered && /-r0$/.test(r.afterHome), JSON.stringify([r.active, r.picked, r.afterHome]));
  check(name, "toasts: errors in role=alert and stay, success auto-dismisses, close works", r.alertToast === 1 && r.statusToast === 1 && r.errorStays === 1 && r.okGone === 0 && r.errorClosed === 0 && r.regions[0] === "polite" && r.regions[1] === "alert", JSON.stringify(r));
  check(name, "dialog: opens modal with focus inside", r.dialogOpen && r.dialogFocus === "INPUT");
  await d.page.key("Enter"); await sleep(80);
  const err = await d.page.eval(`document.querySelector('dialog .err')?.textContent`);
  check(name, "dialog: Enter with invalid input shows the error and stays open", err === "Escribe un nombre." && await d.page.eval(`!!document.querySelector('dialog[open]')`), err);
  await d.page.key("Escape"); await sleep(120);
  const back = await d.page.eval(`({ open: !!document.querySelector('dialog'), result: window.__dlgResult, focus: document.activeElement?.textContent })`);
  check(name, "dialog: Esc cancels (null) and focus returns to the opener", !back.open && back.result === null && back.focus === "abrir", JSON.stringify(back));
  await d.page.eval(`document.querySelectorAll('button').forEach(b => b.textContent === 'abrir' && b.remove())`);
  check(name, "components: no JS errors", d.errors.length === mark, d.errors.slice(mark).join(" | "));
}

async function walkApp(chrome, name, port) {
  console.log(`\n== ${name}/ new UI (port ${port})`);
  const d = await Driver.open(chrome.port, `${name}/new`);
  try {
    const cat = await api(port, "/api/catalog");
    await d.page.send("Page.addScriptToEvaluateOnNewDocument", { source: APP_PROBE });
    await d.goto(`http://127.0.0.1:${port}/`);
    const ready = await d.waitFor(`() => document.querySelector('#main [data-view="home"][data-ready]')`);
    await d.settle({ quiet: 300 });
    check(name, "app: home drawn", ready, "the new home never appeared");
    check(name, "app: project name in the top bar", (await d.page.eval(`document.getElementById("kiln-project").textContent`)) === cat.project);
    check(name, "app: shell has top bar, nav, main, search, Comandos, Releer", await d.page.eval(`!!(document.querySelector('header.topbar') && document.querySelector('nav[aria-label=Principal]') && document.querySelector('main#main') && document.querySelector('input[aria-label^=Buscar]') && [...document.querySelectorAll('button')].some(b => b.textContent.startsWith('Comandos')) && [...document.querySelectorAll('button')].some(b => b.textContent === 'Releer'))`));

    const nav = await d.page.eval(`Object.fromEntries([...document.querySelectorAll('nav a[data-nav]')].map(a => [a.dataset.nav, a.querySelector('.cnt').textContent]))`);
    const elements = cat.content.reduce((n, s) => n + s.elements.length, 0);
    check(name, "nav: entries present (Contenido only with content sets)", ["home", "context", "tree", "catalog", "materials", "jobs"].every(v => v in nav) && ("content" in nav) === (cat.content.length > 0), Object.keys(nav).join(","));
    check(name, "nav: counters match the catalog", nav.catalog === plural(cat.pieces.length) && nav.materials === plural(cat.materials.length) && nav.tree === plural(Object.keys(cat.tree.nodes).length) && (!cat.content.length || nav.content === plural(elements)), JSON.stringify(nav));
    const dots = await d.page.eval(`Object.fromEntries([...document.querySelectorAll('nav a[data-nav]')].map(a => [a.dataset.nav, a.querySelector('.dot').className]))`);
    const worst = pred => { const f = cat.findings.filter(pred); return f.some(x => x.severity === "error") ? "error" : f.some(x => x.severity === "warn") ? "warn" : f.length ? "info" : "none"; };
    const pieceKeys = new Set(cat.pieces.map(p => p.key)), kitNames = new Set(cat.kits.map(k => k.name));
    check(name, "nav: catalog dot is the worst severity of its findings", dots.catalog.endsWith(worst(f => pieceKeys.has(f.subject) || kitNames.has(f.subject))), `${dots.catalog}`);

    const tiles = await d.page.eval(`Object.fromEntries([...document.querySelectorAll('.tile')].map(t => [t.querySelector('.l').textContent, t.querySelector('.v').textContent]))`);
    check(name, "home: shows counts from the catalog", tiles.Kits === plural(cat.kits.length) && tiles.Piezas === plural(cat.pieces.length) && tiles.Materiales === plural(cat.materials.length) && (!cat.content.length || tiles.Elementos === plural(elements)), JSON.stringify(tiles));
    if (cat.content.length) {
      const rowsText = await d.page.eval(`[...document.querySelectorAll('.set-row')].map(r => r.textContent)`);
      check(name, "home: shows each content set with its status", cat.content.every(s => rowsText.some(t => t.includes(s.name) && /validado|error|aviso/.test(t))), rowsText.join(" | "));
    }
    const sev = ["error", "warn", "info"].filter(s => cat.findings.some(f => f.severity === s));
    const groups = await d.page.eval(`[...document.querySelectorAll('.issues .gh .t')].map(e => e.textContent)`);
    const hasFail = cat.findings.some(f => f.severity !== "info");
    if (hasFail) {
      check(name, "home: findings grouped by severity with their totals", groups.length === sev.length && sev.every((s, i) => groups[i].startsWith(plural(cat.findings.filter(f => f.severity === s).length))), groups.join(" | "));
    } else check(name, "home: no errors or warnings shows the calm state", await d.page.eval(`!!document.querySelector('.calm')`));
    const next = await d.page.eval(`document.querySelector('[data-next]')?.dataset.next`);
    const expectNext = cat.findings.some(f => f.severity === "error") ? "errors" : cat.findings.some(f => f.severity === "warn") ? "warnings" : (!cat.kits.length && !cat.content.length) ? "no-kits" : "ok";
    check(name, "home: next-step card follows the rules", next === expectNext, `${next} vs ${expectNext}`);
    check(name, "home: no JS errors", d.errors.length === 0, d.errors.join(" | "));
    await appShot(d, `${name}-app-home.png`);
    const classicLink = await d.page.eval(`document.querySelector('nav a[href="/classic"]')?.textContent.trim()`);
    check(name, "routes: the new UI links to the previous one (Interfaz anterior -> /classic)", classicLink === "Interfaz anterior", String(classicLink));
    const routes = await d.page.eval(`(async () => { const get = async p => { const r = await fetch(p); return { status: r.status, text: await r.text() }; };
      const [root, app, classic] = await Promise.all([get('/'), get('/app'), get('/classic')]);
      return { same: root.text === app.text, rootIsNew: root.text.includes('/ui/app/main.js'), classicOld: !classic.text.includes('/ui/app/main.js') && classic.text.includes('Interfaz anterior: se retirará en la próxima entrega') && classic.text.includes('href="/"'), statuses: [root.status, app.status, classic.status] }; })()`);
    check(name, "routes: / serves the new UI, /app the same page, /classic the old one with its banner", routes.same && routes.rootIsNew && routes.classicOld && routes.statuses.every(s => s === 200), JSON.stringify(routes));

    // 900 px: nothing overflows horizontally
    await d.page.send("Emulation.setDeviceMetricsOverride", { width: 900, height: 800, deviceScaleFactor: 1, mobile: false });
    await sleep(250);
    const over = await d.page.eval(`({ sw: document.documentElement.scrollWidth, iw: innerWidth, main: document.getElementById('main').scrollWidth - document.getElementById('main').clientWidth })`);
    check(name, "app: usable at 900 px (no horizontal overflow)", over.sw <= over.iw && over.main <= 1, JSON.stringify(over));
    await appShot(d, `${name}-app-home-w900.png`);
    await d.page.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });

    // a finding links to its element; reload keeps the route
    const first = await d.page.eval(`document.querySelector('.issue')?.getAttribute('href') ?? null`);
    if (first) {
      await d.clickAndSettle(`() => document.querySelector('.issue')`);
      const hash = await d.page.eval("location.hash");
      check(name, "home: clicking a finding navigates (hash changes to its route)", hash === first && hash !== "" && hash !== "#/home", `${hash} vs ${first}`);
      check(name, "app: the nav marks the current view", await d.page.eval(`!!document.querySelector('nav a[aria-current=page]')`));
      await d.page.eval("location.reload()");
      const back = await d.waitFor(`() => document.querySelector('#main [data-view]') && location.hash === ${JSON.stringify(first)}`);
      await d.settle({ quiet: 300 });
      check(name, "app: reload keeps the route", back && (await d.page.eval("location.hash")) === first);
    }
    await d.goto(`http://127.0.0.1:${port}/#/home`);
    await d.waitFor(`() => document.querySelector('#main [data-view="home"][data-ready]')`);
    await d.settle({ quiet: 300 });
    const mark = d.errors.length;
    await d.clickAndSettle(`() => document.getElementById('btnReleer')`);
    check(name, "app: Releer answers with a toast and gives the button back", await d.page.eval(`!!document.querySelector('.toast.ok') && !document.getElementById('btnReleer').disabled`));
    check(name, "app: Releer no JS errors", d.errors.length === mark, d.errors.slice(mark).join(" | "));
    await walkAppViews({ d, name, port, cat, check, note, shot: appShot });
    await walkNewViews({ d, name, port, cat, check, note, shot: appShot });
    await walkAppA11y({ d, name, port, cat, check, note, shot: async (dd, file) => appShot(dd, file) });
    if (name === "examples") await walkComponents(d, name);
    check(name, "app: ran without JS errors", d.errors.length === 0, d.errors.join(" | "));
  } catch (e) {
    check(name, "app: ran to the end", false, e.stack?.split("\n").slice(0, 3).join(" "));
  } finally { await d.close(); }
}

async function measureAppOnce(chrome, port) {
  const d = await Driver.open(chrome.port, "measure-app");
  try {
    await d.page.send("Page.addScriptToEvaluateOnNewDocument", { source: APP_PROBE });
    await d.goto(`http://127.0.0.1:${port}/`);
    await d.waitFor(`() => window.__app?.paint != null`, 60000);
    await d.settle({ quiet: 300 });
    const t = await d.page.eval("({ ready: __app.ready, paint: __app.paint })");
    return { app_home_dom_ms: t.ready, app_home_paint_ms: t.paint, app_home_nodes: await d.domCount(), app_js_errors: d.errors.length };
  } finally { await d.close(); }
}

async function measureApp(chrome, port) {
  const runs = [];
  for (let i = 0; i < RUNS; i++) runs.push(await measureAppOnce(chrome, port));
  return Object.fromEntries(Object.keys(runs[0]).map(k => [k, median(runs.map(r => r[k]))]));
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
  // earlier checks already warmed the server cache, so the cold load is forced with the explicit marker "Releer" sends
  const t0 = performance.now(); const cat = await (await fetch(`http://127.0.0.1:${port}/api/catalog`, { headers: { "X-Kiln-Fresh": "1" } })).json(); const cold_ms = performance.now() - t0;
  const kit = cat.kits[Math.min(3, cat.kits.length - 1)].name;
  const runs = [];
  for (let i = 0; i < RUNS; i++) { runs.push(await measureOnce(chrome, port, kit)); console.log(`  run ${i + 1} done`); }
  const api_ms = [], bytes = [];
  for (let i = 0; i < RUNS; i++) { const t = performance.now(); const r = await fetch(`http://127.0.0.1:${port}/api/catalog`); const b = (await r.arrayBuffer()).byteLength; api_ms.push(performance.now() - t); bytes.push(b); }
  // the same request with the validator the page sends on every later read: the server answers 304
  const etag = (await fetch(`http://127.0.0.1:${port}/api/catalog`)).headers.get("etag"), cached_ms = [];
  for (let i = 0; i < RUNS; i++) { const t = performance.now(); const r = await fetch(`http://127.0.0.1:${port}/api/catalog`, { headers: etag ? { "If-None-Match": etag } : {} }); await r.arrayBuffer(); cached_ms.push(performance.now() - t); }
  const out = { api_catalog_ms: cold_ms, api_catalog_warm_ms: median(api_ms), api_catalog_cached_ms: median(cached_ms), api_catalog_kb: median(bytes) / 1024, pieces: cat.pieces.length, findings: cat.findings.length };
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
  const SPECIAL = new Set(["longnames"]);
  for (const [name, port] of Object.entries(targets)) if (!SPECIAL.has(name)) await walk(chrome, name, port);
  for (const [name, port] of Object.entries(targets)) if (name !== "wizard" && !SPECIAL.has(name)) await walkApp(chrome, name, port);
  if (targets.wizard) await walkAppWizard({ Driver, chrome, name: "wizard", port: targets.wizard, check, shot: (d, n) => shot(d, "wizard", n) });
  if (targets.longnames) {
    console.log(`\n== longnames (port ${targets.longnames})`);
    const d = await Driver.open(chrome.port, "longnames");
    try { await d.goto(`http://127.0.0.1:${targets.longnames}/#/catalog`); await d.waitFor(`() => document.querySelector('#main [data-view="catalog"][data-ready]')`); await d.settle({ quiet: 300 }); await walkLongNames({ d, name: "longnames", port: targets.longnames, check }); }
    finally { await d.close(); }
  }
  if (targets.synthetic) { perf = await measure(chrome, targets.synthetic); Object.assign(perf.median, await measureApp(chrome, targets.synthetic)); Object.assign(perf.median, await measureAppViews({ chrome, port: targets.synthetic, Driver, cat: await api(targets.synthetic, '/api/catalog') })); }
} finally { chrome.kill(); }

// ---- R15 performance budget of the new UI: a failing condition from now on
// The limits come from the spec (R15). Headless Chrome on a busy machine is
// noisy, so a metric fails only above LIMIT x TOLERANCE; between the limit and
// that, it is reported as a note. DOM node counts are deterministic: no tolerance.
const TOLERANCE = 1.25;
const BUDGET = [
  ["app_home_paint_ms", 300, "first view (home, painted)", true],
  ["app_catalog_first_ms", 300, "first view straight on the catalog", true],
  ["app_catalog_filter_ms", 50, "catalog filter keystroke", true],
  ["app_catalog_rows", 80, "row nodes in the catalog (5,000 pieces)", false],
];
if (perf) for (const [key, limit, label, noisy] of BUDGET) {
  const v = perf.median[key], max = noisy ? limit * TOLERANCE : limit;
  check("synthetic", `budget: ${label} < ${limit}${noisy ? ` (x${TOLERANCE} tolerance)` : ""}`, Number.isFinite(v) && v > 0 && v < max, `${key} = ${v == null ? "missing" : v.toFixed(1)}`);
  if (noisy && v != null && v >= limit && v < max) note("synthetic", `budget: ${key} = ${v.toFixed(0)} is over ${limit} but inside the tolerance`);
}

// a lower bound too: a zero, NaN or missing metric of the new UI is never a pass
if (perf) for (const key of Object.keys(perf.median).filter(k => k.startsWith("app_") && !/errors$/.test(k)))
  check("synthetic", `metric is real: ${key} > 0`, Number.isFinite(perf.median[key]) && perf.median[key] > 0, `${key} = ${perf.median[key]}`);
if (perf) check("synthetic", "catalog was measured, not the home view: nodes differ", perf.median.app_catalog_nodes !== perf.median.app_home_nodes, `catalog ${perf.median.app_catalog_nodes} vs home ${perf.median.app_home_nodes}`);

// ---- before / after: the previous UI (/classic) against the new one (/), same run, same project
if (perf) {
  const m = perf.median, f1 = v => v == null ? "-" : Math.abs(v) >= 100 ? v.toFixed(0) : v.toFixed(1);
  const rows = [
    ["first view (painted), ms", m.fmp_paint_ms, m.app_home_paint_ms],
    ["switch to catalog, ms", m.switch_catalog_ms, m.app_catalog_switch_ms],
    ["filter keystroke in catalog, ms", m.keystroke_ms, m.app_catalog_filter_ms],
    ["global search keystroke, ms", m.keystroke_ms, m.app_search_ms],
    ["DOM nodes, catalog view", m.dom_catalog, m.app_catalog_nodes],
    ["DOM nodes, home view", m.dom_default, m.app_home_nodes],
    ["/api/catalog cold (forced fresh read, X-Kiln-Fresh), ms", m.api_catalog_ms, m.api_catalog_ms],
    ["/api/catalog cached (304 with If-None-Match), ms", m.api_catalog_warm_ms, m.api_catalog_cached_ms],
    ["JS heap after the views, MB", m.heap_after_views_mb, m.app_heap_mb],
  ];
  // the previous UI as first recorded (before the server cached the catalog), kept in the baseline as `reference`
  const ref = (fs.existsSync(BASELINE) ? JSON.parse(fs.readFileSync(BASELINE, "utf8")) : {}); const R = ref.reference ?? ref.metrics ?? {};
  const refOf = { 0: R.fmp_paint_ms, 1: R.switch_catalog_ms, 2: R.keystroke_ms, 3: R.keystroke_ms, 4: R.dom_catalog, 5: R.dom_default, 6: R.api_catalog_ms, 7: R.api_catalog_ms, 8: R.heap_after_views_mb };
  const lines = ["| measure | previous UI, first recorded | previous UI, this run | new UI, this run |", "|---|---:|---:|---:|", ...rows.map((r, i) => `| ${r[0]} | ${f1(refOf[i])} | ${f1(r[1])} | ${f1(r[2])} |`)];
  console.log("\n== before / after (synthetic project: " + m.pieces + " pieces, " + m.findings + " findings)\n" + lines.join("\n"));
  fs.writeFileSync(path.join(OUT, "comparison.md"), lines.join("\n") + "\n");
}

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
  const prev = fs.existsSync(BASELINE) ? JSON.parse(fs.readFileSync(BASELINE, "utf8")) : {};
  fs.writeFileSync(BASELINE, JSON.stringify({ chrome: chrome.version, checks, metrics: perf?.median ?? {}, reference: prev.reference ?? prev.metrics ?? {}, notes }, null, 2) + "\n");
  console.log(`\nbaseline written: ${BASELINE}`);
}
console.log(`\n${results.length - failed.length} checks passed, ${failed.length} failed, ${notes.length} notes`);
for (const f of failed) console.log(`FAIL ${f.id}: ${f.detail}`);
for (const r of regressions) console.log(`REGRESSION ${r}`);
process.exit(failed.length || regressions.length ? 1 : 0);
