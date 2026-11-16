// Walkthrough of the data views of the new UI (/) (Catálogo, Materiales, Contenido,
// Árbol), the global search and the command palette; and their measurements.
import { sleep } from "./cdp.mjs";
import { S } from "./selectors.mjs";

const q = JSON.stringify;
const num = s => Number(String(s).replace(/\D/g, ""));
const norm = s => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
const enc = encodeURIComponent;
const pieceHash = p => `#/catalog/${enc(p.kit)}/${enc(p.key.startsWith(p.kit + "/") ? p.key.slice(p.kit.length + 1) : p.key)}`;

const text = (d, sel) => d.page.eval(`document.querySelector(${q(sel)})?.innerText ?? ''`);
const count = (d, sel) => d.page.eval(`document.querySelectorAll(${q(sel)}).length`);

async function go(d, port, hash, view) {
  await d.goto(`http://127.0.0.1:${port}/${hash}`);
  await d.waitFor(`() => document.querySelector('#main [data-view=${q(view)}][data-ready]')`);
  await d.settle({ quiet: 250 });
}
async function reload(d, view) {
  await d.page.eval("location.reload()");
  await sleep(200);
  await d.waitFor(`() => document.querySelector('#main [data-view=${q(view)}][data-ready]')`);
  await d.settle({ quiet: 250 });
}
const typeInto = async (d, sel, str) => {
  await d.page.eval(`(() => { const el = document.querySelector(${q(sel)}); el.focus(); el.setSelectionRange(0, el.value.length); })()`);
  await d.page.key("Backspace");
  await d.type(str);
  await d.settle();
};

export async function walkAppViews({ d, name, port, cat, ctx, check, note, shot }) {
  const errs = () => d.errors.length;
  const hay = p => norm(`${p.name} ${p.key} ${p.kit} ${(p.material_slots ?? []).map(s => p.bindings?.[s] || s).join(" ")}`);
  const worstOfPiece = p => { const f = cat.findings.filter(x => x.subject === p.key).map(x => x.severity); return f.includes("error") ? "error" : f.includes("warn") ? "warn" : f.length ? "info" : null; };

  // ---------------------------------------------------------------- Catálogo
  if (cat.pieces.length) {
    const mark = errs();
    await go(d, port, "#/catalog", "catalog");
    check(name, "catalog: one link per kit", (await count(d, '#cKits a[data-kit]:not([data-kit=""])')) === cat.kits.length);
    check(name, "catalog: the counter matches the catalog", num(await text(d, "#cCount b")) === cat.pieces.length, await text(d, "#cCount"));
    const rows = await count(d, ".vrow");
    check(name, "catalog: only a window of rows is in the DOM (< 80)", rows > 0 && rows < 80 && rows <= cat.pieces.length, `${rows} rows`);
    await shot(d, `${name}-app-catalog.png`);

    const pick = cat.pieces[Math.floor(cat.pieces.length / 2)];
    const want = cat.pieces.filter(p => hay(p).includes(norm(pick.name))).length;
    await typeInto(d, "#cQ", pick.name);
    check(name, "catalog: the filter narrows and the counter says how many", num(await text(d, "#cCount b")) === want && want < cat.pieces.length || cat.pieces.length === 1, `${await text(d, "#cCount")} vs ${want}`);
    check(name, "catalog: the filter is kept in the URL", (await d.page.eval("location.hash")).includes("q=" + enc(pick.name)));

    const first = await d.page.eval(`document.querySelector('.vrow .vc')?.textContent ?? ''`);
    await d.clickAndSettle(`() => document.querySelector('.vrow')`);
    const det = await text(d, "#detail");
    check(name, "catalog: selecting a row fills the panel (data, findings)", det.includes(first) && /hallazgos de esta pieza/i.test(det) && /datos/i.test(det), det.slice(0, 80));
    const hash = await d.page.eval("location.hash");
    check(name, "catalog: the selection is in the URL", hash.startsWith("#/catalog/"), hash);
    await reload(d, "catalog");
    check(name, "catalog: the deep link to a piece survives a reload", (await text(d, "#detail")).includes(first) && (await count(d, ".vrow.sel")) === 1, hash);

    // severity chips and kits
    await go(d, port, "#/catalog", "catalog");
    const sev = ["error", "warn", "info"].find(s => cat.pieces.some(p => worstOfPiece(p) === s));
    if (sev) {
      await d.clickAndSettle(`() => document.querySelector('button[data-sev=${q(sev)}]')`);
      const n = cat.pieces.filter(p => worstOfPiece(p) === sev).length;
      check(name, `catalog: the ${sev} chip keeps only those pieces`, num(await text(d, "#cCount b")) === n && (await d.page.eval("location.hash")).includes("sev=" + sev), `${await text(d, "#cCount")} vs ${n}`);
      await d.clickAndSettle(`() => document.querySelector('button[data-sev=${q(sev)}]')`);
    } else note(name, "catalog chips: no piece has findings");
    const kit = cat.kits[0];
    await d.clickAndSettle(`() => document.querySelector('#cKits a[data-kit=${q(kit.name)}]')`);
    check(name, "catalog: choosing a kit lists only its pieces", num(await text(d, "#cCount b")) === cat.pieces.filter(p => p.kit === kit.name).length && (await d.page.eval(`document.querySelector('#cKits a[aria-current]')?.dataset.kit`)) === kit.name);
    check(name, "catalog: the kit panel shows its folder", /carpeta/i.test(await text(d, "#detail")) || !(await d.page.eval("document.getElementById('shell').classList.contains('with-detail')")) );
    await d.clickAndSettle(`() => document.querySelector('.vh button[data-key="name"]')`);
    check(name, "catalog: sorting a column is kept in the URL", (await d.page.eval("location.hash")).includes("sort=name"));

    // actions of a piece: a failing request says what happened and gives the control back
    const slotted = cat.pieces.find(p => (p.material_slots ?? []).length && cat.materials.length);
    if (slotted) {
      await go(d, port, pieceHash(slotted), "catalog");
      await d.page.eval(`document.querySelectorAll('.toast').forEach(t => t.remove())`);
      await d.page.eval(`(() => { window.__real = window.fetch; window.fetch = (u, o) => String(u).includes('/api/bind') ? Promise.resolve(new Response(JSON.stringify({ error: 'prueba de error' }), { status: 400, headers: { 'content-type': 'application/json' } })) : window.__real(u, o);
        const s = document.querySelector('#detail select[data-slot]'); const o = document.createElement('option'); o.value = '__x'; o.textContent = 'x'; s.appendChild(o); s.value = '__x'; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
      const failed = await d.waitFor(`() => document.querySelector('.toast.error')`, 5000);
      const msg = await text(d, ".toast.error");
      check(name, "catalog: a failing bind shows a clear error toast", failed && /prueba de error/.test(msg) && /No se pudo/.test(msg), msg);
      await d.page.eval("window.fetch = window.__real");
      if (name === "synthetic") {
        await go(d, port, pieceHash(slotted), "catalog");
        await d.page.eval(`document.querySelectorAll('.toast').forEach(t => t.remove())`);
        const material = cat.materials[0].key, slot = slotted.material_slots[0];
        await d.page.eval(`(() => { const s = document.querySelector('#detail select[data-slot=${q(slot)}]'); s.value = ${q(material)}; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
        const ok = await d.waitFor(`() => document.querySelector('.toast.ok')`, 15000);
        await d.settle({ quiet: 300 });
        check(name, "catalog: binding a slot answers with a toast and the panel shows it", ok && (await d.page.eval(`document.querySelector('#detail select[data-slot=${q(slot)}]')?.value`)) === material);
        await d.page.eval(`document.querySelectorAll('.toast').forEach(t => t.remove())`);
        await d.page.eval(`(() => { const s = document.querySelector('#detail select[data-slot=${q(slot)}]'); s.value = ''; s.dispatchEvent(new Event('change', { bubbles: true })); })()`);
        await d.waitFor(`() => document.querySelector('.toast.ok')`, 15000);
        await d.settle({ quiet: 300 });
      }
    } else note(name, "catalog actions: no piece with material slots");
    check(name, "catalog: no JS errors", errs() === mark, d.errors.slice(mark).join(" | "));
    // 900 px
    await go(d, port, "#/catalog", "catalog");
    await d.page.send("Emulation.setDeviceMetricsOverride", { width: 900, height: 800, deviceScaleFactor: 1, mobile: false });
    await sleep(300);
    const over = await d.page.eval(`({ sw: document.documentElement.scrollWidth, iw: innerWidth })`);
    check(name, "catalog: usable at 900 px (no horizontal overflow)", over.sw <= over.iw, JSON.stringify(over));
    await shot(d, `${name}-app-catalog-w900.png`);
    await d.page.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  }

  // -------------------------------------------------------------- Materiales
  {
    const mark = errs();
    await go(d, port, "#/materials", "materials");
    if (!cat.materials.length) {
      check(name, "materials: the empty state is explained", (await text(d, "#main")).includes("Todavía no hay materiales"));
    } else {
      check(name, "materials: the list is not cut (counter = every material)", num(await text(d, "#mCount b")) === cat.materials.length, await text(d, "#mCount"));
      const rows = await count(d, ".vrow");
      check(name, "materials: a window of rows (< 80)", rows > 0 && rows < 80, `${rows}`);
      const m = cat.materials[Math.floor(cat.materials.length / 2)];
      const want = cat.materials.filter(x => norm(`${x.key} ${x.name} ${x.file} ${x.source}`).includes(norm(m.key))).length;
      await typeInto(d, "#mQ", m.key);
      check(name, "materials: the filter narrows", num(await text(d, "#mCount b")) === want, `${await text(d, "#mCount")} vs ${want}`);
      await d.clickAndSettle(`() => document.querySelector('.vrow')`);
      const det = await text(d, "#detail");
      check(name, "materials: selecting a row shows the material, where it is used and its findings", /datos/i.test(det) && /ranuras que la usan/i.test(det) && /hallazgos de este material/i.test(det), det.slice(0, 80));
      const hash = await d.page.eval("location.hash");
      await reload(d, "materials");
      check(name, "materials: the deep link survives a reload", hash.startsWith("#/materials/") && /datos/i.test(await text(d, "#detail")), hash);
      await shot(d, `${name}-app-materials.png`);
    }
    check(name, "materials: no JS errors", errs() === mark, d.errors.slice(mark).join(" | "));
  }

  // ---------------------------------------------------------------- Contenido
  if (cat.content.length) {
    const mark = errs(), set = cat.content[0];
    await go(d, port, "#/content", "content");
    check(name, "content: shows the set and one row per element (counter)", (await text(d, ".sethead h2")) === set.name && num(await text(d, "#eCount b")) === set.elements.length, await text(d, "#eCount"));
    check(name, "content: one chip per kind", (await count(d, "#eChips button")) === set.kinds.length);
    const subject = e => `${set.name}:${e.file}#${e.id}`;
    const own = f => f.subject === set.name || f.subject.startsWith(set.name + ":");
    const loose = cat.findings.filter(f => own(f) && !set.elements.some(e => subject(e) === f.subject));
    const anyFinding = cat.findings.some(own);
    if (!anyFinding) check(name, "content: a set without findings shows the calm state", (await count(d, "[data-calm]")) === 1);
    check(name, "content: the set-level findings are shown", loose.length === 0 || (await count(d, ".loose")) === 1);
    const bad = set.elements.find(e => cat.findings.some(f => f.subject === subject(e)));
    if (bad) {
      await typeInto(d, "#eQ", bad.id);
      check(name, "content: a row with findings shows them", (await count(d, ".vrow .sev.error, .vrow .sev.warn, .vrow .sev.info")) >= 1);
    }
    const target = bad ?? set.elements[0];
    await typeInto(d, "#eQ", target.id);
    const narrowed = num(await text(d, "#eCount b")), expected = set.elements.filter(e => norm(`${e.kind} ${e.id} ${e.file}`).includes(norm(target.id))).length;
    check(name, "content: the filter narrows the rows", narrowed === expected && (set.elements.length === 1 || narrowed < set.elements.length), `${narrowed} vs ${expected}`);
    await d.clickAndSettle(`() => document.querySelector('.vrow')`);
    check(name, "content: selecting a row fills the panel", /comprobaciones de este elemento/i.test(await text(d, "#detail")));
    const hash = await d.page.eval("location.hash");
    await reload(d, "content");
    check(name, "content: the deep link to an element survives a reload", hash.startsWith("#/content/") && (await count(d, ".vrow.sel")) === 1, hash);
    await go(d, port, "#/content", "content");
    await shot(d, `${name}-app-content.png`);
    check(name, "content: no JS errors", errs() === mark, d.errors.slice(mark).join(" | "));
  }

  // -------------------------------------------------------------------- Árbol
  if (cat.tree) {
    const mark = errs(), nodes = Object.keys(cat.tree.nodes).length, root = cat.tree.nodes[cat.tree.root];
    await go(d, port, "#/tree", "tree");
    const rows = await count(d, "#tOutline .trow");
    check(name, "tree: the root and its children are listed", rows >= 1 + root.children.length && (await text(d, "#tOutline")).includes(root.name), `${rows} rows`);
    check(name, "tree: only expanded branches are in the DOM", rows <= nodes && (nodes < 30 || rows < nodes), `${rows} of ${nodes}`);
    check(name, "tree: the panel shows the root", (await text(d, "#detail")).includes(root.name) || !(await d.page.eval("document.getElementById('shell').classList.contains('with-detail')")));
    const closed = await d.page.eval(`!!document.querySelector('#tOutline .trow[aria-expanded="false"]')`);
    if (closed) {
      await d.clickAndSettle(`() => document.querySelector('#tOutline .trow[aria-expanded="false"] .ttog')`);
      check(name, "tree: expanding a node adds its children", (await count(d, "#tOutline .trow")) > rows);
    }
    await go(d, port, "#/tree", "tree");
    const child = root.children[0];
    if (child) {
      const n = cat.tree.nodes[child];
      await d.clickAndSettle(`() => [...document.querySelectorAll('#tOutline .trow')].find(r => r.dataset.key === ${q(child)})`);
      const hash = await d.page.eval("location.hash");
      check(name, "tree: selecting a node fills the panel, the breadcrumb and the URL", (await text(d, "#detail")).includes(n.name) && (await text(d, "#tCrumbs")).includes(n.name) && hash.startsWith("#/tree/"), hash);
      await reload(d, "tree");
      check(name, "tree: the deep link survives a reload", (await count(d, `#tOutline .trow.sel`)) === 1 && (await text(d, "#detail")).includes(n.name), hash);
    }
    const sub = Object.values(cat.tree.nodes).find(n => !n.children.length && n.status === "spec");
    if (sub) {
      await go(d, port, `#/tree/${enc(sub.key)}`, "tree");
      check(name, "tree: a leaf with a spec links to it", (await d.page.eval(`document.querySelector('#detail a[href^="#/spec/"]')?.textContent ?? ''`)) === "Ver la ficha");
    }
    const missing = Object.values(cat.tree.nodes).find(n => n.level === "asset" && n.status === "missing");
    if (missing) {
      await go(d, port, `#/tree/${enc(missing.key)}`, "tree");
      check(name, "tree: a missing asset offers Pedir ficha", (await count(d, "#detail button[data-ask]")) === 1);
    }
    await d.clickAndSettle(`() => document.getElementById('tExpand')`);
    check(name, "tree: Expandir todo lists every node", (await count(d, "#tOutline .trow")) >= nodes, `${await count(d, "#tOutline .trow")} of ${nodes}`);
    await d.clickAndSettle(`() => document.getElementById('tCollapse')`);
    check(name, "tree: Plegar todo leaves the root", (await count(d, "#tOutline .trow")) === 1);
    await go(d, port, "#/tree", "tree");
    await shot(d, `${name}-app-tree.png`);
    check(name, "tree: no JS errors", errs() === mark, d.errors.slice(mark).join(" | "));
  }

  // ------------------------------------------------- global search and palette
  {
    const mark = errs();
    await go(d, port, "#/home", "home");
    const probe = cat.pieces[0];
    if (probe) {
      const tok = norm(probe.name).slice(0, Math.min(5, probe.name.length));
      const total = cat.pieces.filter(p => norm(`${p.name} ${p.key}`).includes(tok)).length;
      await d.focusSearch();
      await d.type(tok);
      await sleep(350); await d.settle();
      const open = await d.page.eval(`!document.getElementById('gres').hidden`);
      const head = await d.page.eval(`[...document.querySelectorAll('#gres h3')].map(h => h.textContent.trim())`);
      check(name, "search: results are grouped by type with the total of each", open && head.some(h => h.startsWith("Piezas") && num(h.replace("Piezas", "")) === total), JSON.stringify(head));
      const items = await count(d, "#gres [role=group]:first-of-type .opt:not(.all)");
      check(name, "search: each group is capped at 5", items <= 5 && items === Math.min(5, total), `${items}`);
      if (total > 5) check(name, "search: a capped group offers 'Ver todos'", (await count(d, "#gres .opt.all")) >= 1);
      await shot(d, `${name}-app-search.png`);
      await d.page.key("ArrowDown");
      check(name, "search: ArrowDown marks an option (aria-activedescendant)", await d.page.eval(`document.getElementById('gq').getAttribute('aria-activedescendant') === 'sr-0' && document.getElementById('sr-0').getAttribute('aria-selected') === 'true'`));
      await d.page.key("Enter");
      await d.settle({ quiet: 300 });
      const hash = await d.page.eval("location.hash");
      check(name, "search: Enter opens the option", hash.startsWith("#/catalog/") && (await d.page.eval(`document.getElementById('gres').hidden`)), hash);
      check(name, "search: the opened piece shows in the panel", (await text(d, "#detail")).includes(probe.name) || (await text(d, "#detail")).length > 0);
      await d.focusSearch(); await d.type("zzzqq"); await sleep(350); await d.settle();
      check(name, "search: nothing found says so", (await text(d, "#gres")).includes("Nada coincide"));
      await d.page.key("Escape");
      check(name, "search: Esc closes the results", await d.page.eval(`document.getElementById('gres').hidden`));
      await d.clearSearch(); await d.settle();
    }
    // palette
    await go(d, port, "#/home", "home");
    await d.page.combo("k", 75, 2);
    await d.settle();
    check(name, "palette: Ctrl+K opens the dialog with the views and Releer", await d.page.eval(`!!document.querySelector('dialog[open] .pal') && [...document.querySelectorAll('dialog .opt')].some(o => o.textContent.includes('Catálogo')) && [...document.querySelectorAll('dialog .opt')].some(o => o.textContent.includes('Releer'))`));
    await shot(d, `${name}-app-palette.png`);
    await d.type("mater"); await d.settle();
    await d.page.key("Enter");
    await d.settle({ quiet: 300 });
    check(name, "palette: typing a view and Enter goes there, the dialog closes", (await d.page.eval("location.hash")).startsWith("#/materials") && (await count(d, "dialog")) === 0, await d.page.eval("location.hash"));
    if (probe) {
      await d.page.combo("k", 75, 2);
      await d.settle();
      await d.type(probe.name.slice(0, 6)); await d.settle();
      await d.page.key("ArrowDown"); await d.page.key("ArrowUp");
      await d.page.key("Enter");
      await d.settle({ quiet: 300 });
      const hash = await d.page.eval("location.hash");
      check(name, "palette: a piece name jumps to that piece", hash.startsWith("#/catalog/") && (await text(d, "#detail")).length > 0, hash);
    }
    await d.page.combo("k", 75, 2);
    await d.settle();
    await d.page.key("Escape");
    await sleep(150);
    check(name, "palette: Esc closes it", (await count(d, "dialog")) === 0);
    await d.page.combo("k", 75, 2);
    await d.settle();
    await d.type("releer"); await d.settle();
    await d.page.key("Enter");
    await d.waitFor(`() => document.querySelector('.toast.ok')`, 15000);
    check(name, "palette: the Releer action runs (toast)", await d.page.eval(`!!document.querySelector('.toast.ok')`));
    check(name, "palette: no JS errors", errs() === mark, d.errors.slice(mark).join(" | "));
  }
}

// ------------------------------------------------------------- measurements
const FIRST = `(() => { const k = window.__first = { t: null };
  new MutationObserver(() => { if (k.t == null && document.querySelector('#main [data-view="catalog"] .vrow')) { k.t = performance.now(); } }).observe(document, { subtree: true, childList: true });
})()`;

export async function measureAppViews({ chrome, port, Driver, cat }) {
  const m = {};
  // first view: a cold load straight on the catalog
  {
    const d = await Driver.open(chrome.port, "measure-app-first");
    try {
      await d.page.send("Page.addScriptToEvaluateOnNewDocument", { source: FIRST });
      await d.goto(`http://127.0.0.1:${port}/#/catalog`);
      await d.waitFor(`() => window.__first?.t != null`, 60000);
      await d.settle({ quiet: 300 });
      m.app_catalog_first_ms = await d.page.eval("__first.t");
    } finally { await d.close(); }
  }
  const d = await Driver.open(chrome.port, "measure-app-views");
  try {
    await d.goto(`http://127.0.0.1:${port}/#/home`);
    await d.waitFor(`() => document.querySelector('#main [data-view="home"][data-ready]')`, 60000);
    await d.settle({ quiet: 300 });
    m.app_home_nodes = await d.domCount();
    for (const view of ["catalog", "materials", "content", "tree"]) {
      if (view === "content" && !cat.content.length) continue;
      const s = await d.clickAndSettle(`() => document.querySelector('nav a[data-nav=${q(view)}]')`);
      m[`app_${view}_switch_ms`] = s.ms;
      m[`app_${view}_nodes`] = await d.domCount();
      m[`app_${view}_rows`] = await count(d, ".vrow, #tOutline .trow");
      if (view === "catalog") {
        await d.waitFor(`() => document.getElementById('cQ')`);
        await d.settle({ quiet: 200 });
        await d.page.eval(`document.getElementById('cQ').focus()`);
        await d.type("piece0");
        await d.settle();
        await d.arm(); await d.page.key("0");
        const k = await d.settle();
        m.app_catalog_filter_ms = k.ms;
        m.app_catalog_filter_rows = await count(d, ".vrow");
        // the global search, including its 120 ms wait
        await d.focusSearch();
        await d.type("piece0");
        await sleep(400); await d.settle({ quiet: 300 });
        await d.arm(); await d.page.key("0");
        await sleep(400);
        const g = await d.settle({ quiet: 300 });
        m.app_search_ms = g.ms;
        m.app_search_nodes = await count(d, "#gres *");
        await d.clearSearch(); await d.settle();
      }
    }
    m.app_heap_mb = await d.heapMB();
    m.app_views_js_errors = d.errors.length;
  } finally { await d.close(); }
  return m;
}
