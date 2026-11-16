// Accessibility walkthrough of the new UI (R11): a keyboard-only path, the
// roles and names of every view, focus rings, and the 900 px layout.
import { sleep } from "./cdp.mjs";

const q = JSON.stringify;
const VIEWS = ["home", "context", "tree", "content", "catalog", "materials", "spec", "jobs"];

const FOCUS_INFO = `(() => { const el = document.activeElement; if (!el || el === document.body) return null;
  const cs = getComputedStyle(el);
  return { tag: el.tagName.toLowerCase(), id: el.id, cls: String(el.className).slice(0, 40), name: (el.getAttribute('aria-label') || el.textContent || '').trim().slice(0, 30),
    ring: (cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) > 0) || cs.boxShadow !== 'none',
    grid: el.matches('.vgrid'), cq: el.id === 'cQ', catalogLink: el.matches('a[data-nav=catalog]'), homeLink: el.matches('a[data-nav=home]') }; })()`;

// Everything the page offers to a screen reader or the keyboard, per view.
const AUDIT = `(() => {
  const vis = el => el.getClientRects().length > 0 && getComputedStyle(el).visibility !== 'hidden';
  const native = 'a[href],button,input,select,textarea,summary,label,option,[role=button],[role=link],[role=option],[role=row],[role=gridcell],[role=treeitem],[role=tab],[role=menuitem],[role=columnheader]';
  const out = { clickable: [], unnamed: [], unlabelled: [], skipped: [], h1: 0 };
  const focusableUp = el => { for (let e = el; e && e !== document.body; e = e.parentElement) { if (e.matches(native) || (e.hasAttribute('tabindex') && e.tabIndex >= 0)) return true; if (e.matches('[role=grid],[role=listbox],[role=tree]') ) return true; } return false; };
  for (const el of document.querySelectorAll('body *')) {
    if (!vis(el) || el.closest('dialog:not([open])')) continue;
    if (getComputedStyle(el).cursor === 'pointer' && !focusableUp(el)) out.clickable.push(el.tagName.toLowerCase() + '.' + String(el.className).slice(0, 30));
  }
  const named = el => (el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || el.textContent.trim() || el.title || '').length > 0;
  for (const el of document.querySelectorAll('button,a[href],[role=button],[role=link]')) if (vis(el) && !named(el)) out.unnamed.push(el.outerHTML.slice(0, 70));
  for (const el of document.querySelectorAll('input:not([type=hidden]),select,textarea')) {
    if (!vis(el)) continue;
    if (!(el.labels?.length || el.getAttribute('aria-label') || el.getAttribute('aria-labelledby'))) out.unlabelled.push(el.outerHTML.slice(0, 70));
  }
  let prev = 0;
  for (const h of document.querySelectorAll('h1,h2,h3,h4,h5,h6')) {
    if (!vis(h)) continue;
    const n = Number(h.tagName[1]); if (n === 1) out.h1++;
    if (prev && n > prev + 1) out.skipped.push('h' + prev + ' -> h' + n + ' "' + h.textContent.trim().slice(0, 25) + '"');
    prev = n;
  }
  out.live = { status: document.querySelector('#status[aria-live]')?.getAttribute('aria-live'), toast: document.querySelector('#toastwrap [data-region=status][aria-live]')?.getAttribute('aria-live'), alert: document.querySelector('#toastwrap [data-region=alert]')?.getAttribute('role') };
  out.landmarks = !!(document.querySelector('header') && document.querySelector('nav[aria-label]') && document.querySelector('main'));
  out.overflow = document.documentElement.scrollWidth - innerWidth;
  return out;
})()`;

async function openView(d, view) {
  await d.page.eval(`location.hash = '#/${view}'`);
  await d.waitFor(`() => document.querySelector('#main [data-view]') && document.getElementById('main').getAttribute('aria-busy') === 'false'`);
  await d.settle({ quiet: 250 });
}

export async function walkAppA11y({ d, name, port, cat, check, note, shot }) {
  const mark0 = d.errors.length;
  await d.goto(`http://127.0.0.1:${port}/#/home`);
  await d.waitFor(`() => document.querySelector('#main [data-view="home"][data-ready]')`);
  await d.settle({ quiet: 300 });

  // ---- per-view audit
  const problems = { clickable: [], unnamed: [], unlabelled: [], skipped: [], noH1: [], live: [], overflow: [], ring: [] };
  const views = VIEWS.filter(v => v !== "content" || cat.content.length);
  for (const view of views) {
    await openView(d, view);
    const a = await d.page.eval(AUDIT);
    a.clickable.forEach(x => problems.clickable.push(`${view}:${x}`));
    a.unnamed.forEach(x => problems.unnamed.push(`${view}:${x}`));
    a.unlabelled.forEach(x => problems.unlabelled.push(`${view}:${x}`));
    a.skipped.forEach(x => problems.skipped.push(`${view}:${x}`));
    if (a.h1 !== 1) problems.noH1.push(`${view}:${a.h1}`);
    if (!(a.live.status === "polite" && a.live.toast === "polite" && a.live.alert === "alert" && a.landmarks)) problems.live.push(`${view}:${JSON.stringify(a.live)}`);
    // focus rings: Tab through the first stops of the view
    await d.page.eval(`(document.querySelector('#skip') ).focus()`);
    for (let i = 0; i < 14; i++) {
      await d.page.tab(); await sleep(15);
      const f = await d.page.eval(FOCUS_INFO);
      if (f && !f.ring) problems.ring.push(`${view}:${f.tag}#${f.id}.${f.cls} "${f.name}"`);
    }
  }
  check(name, "a11y: no element with a pointer cursor is out of the keyboard's reach", problems.clickable.length === 0, problems.clickable.slice(0, 6).join(" | "));
  check(name, "a11y: every button and link has an accessible name", problems.unnamed.length === 0, problems.unnamed.slice(0, 4).join(" | "));
  check(name, "a11y: every form input has a label", problems.unlabelled.length === 0, problems.unlabelled.slice(0, 4).join(" | "));
  check(name, "a11y: headings never skip a level", problems.skipped.length === 0, problems.skipped.slice(0, 4).join(" | "));
  check(name, "a11y: exactly one h1 per view", problems.noH1.length === 0, problems.noH1.join(" | "));
  check(name, "a11y: landmarks and live regions (status, toasts, alerts) in every view", problems.live.length === 0, problems.live.slice(0, 3).join(" | "));
  check(name, "a11y: every control reached with Tab shows a focus ring", problems.ring.length === 0, problems.ring.slice(0, 5).join(" | "));

  // ---- the catalog grid exposes sort and size
  await openView(d, "catalog");
  const grid = await d.page.eval(`(() => { const g = document.querySelector('.vgrid'); if (!g) return null; return { rows: g.getAttribute('aria-rowcount'), sorts: [...g.querySelectorAll('[role=columnheader]')].every(h => h.hasAttribute('aria-sort')), n: g.querySelectorAll('[role=columnheader]').length }; })()`);
  if (grid) check(name, "a11y: the catalog grid has aria-rowcount and aria-sort on every column header", /^\d+$/.test(grid.rows ?? "") && Number(grid.rows) === cat.pieces.length && grid.sorts && grid.n > 0, JSON.stringify(grid));

  // ---- keyboard only: Inicio -> Catálogo -> filter -> select -> panel -> Ctrl+K -> view
  if (cat.pieces.length) {
    const lost = [];
    const here = async step => { const f = await d.page.eval(FOCUS_INFO); if (!f) lost.push(step); return f; };
    const tabTo = async (pred, step, max = 80, shift = false) => {
      for (let i = 0; i < max; i++) { await d.page.tab(shift); await sleep(12); const f = await here(`${step}#${i}`); if (f && pred(f)) return f; }
      return null;
    };
    await d.page.eval(`location.hash = '#/home'`);
    await d.waitFor(`() => document.querySelector('#main [data-view="home"][data-ready]')`);
    await d.settle({ quiet: 250 });
    await d.page.eval(`document.activeElement.blur()`);
    const home = await tabTo(f => f.homeLink, "to Inicio");
    check(name, "keyboard: Tab reaches the Inicio entry of the navigation", !!home);
    const cat1 = await tabTo(f => f.catalogLink, "to Catálogo");
    await d.page.key("Enter"); await d.waitFor(`() => document.querySelector('#main [data-view="catalog"][data-ready]')`); await d.settle({ quiet: 250 });
    check(name, "keyboard: Tab and Enter on Catálogo open the catalog", !!cat1 && (await d.page.eval("location.hash")).startsWith("#/catalog"));

    const word = cat.pieces[0].name.toLowerCase().replace(/[^a-z0-9]/g, "").slice(0, 3) || "a";
    const before = await d.page.eval(`document.querySelectorAll('.vrow').length`);
    const cq = await tabTo(f => f.cq, "to filter");
    if (cq) { await d.type(word); await d.settle(); }
    const counter = await d.page.eval(`document.querySelector('.vcount')?.textContent ?? ''`);
    check(name, "keyboard: Tab to the filter and typing narrows the list", !!cq && /\d/.test(counter) && before > 0, counter);

    const g = await tabTo(f => f.grid, "to grid");
    for (let i = 0; i < 2; i++) await d.page.key("ArrowDown");
    await d.page.key("Enter"); await d.settle({ quiet: 250 });
    const open = await d.page.eval(`({ expanded: document.getElementById('btnPanel').getAttribute('aria-expanded'), body: document.querySelector('#detail .dbody')?.innerText.trim().length ?? 0 })`);
    check(name, "keyboard: arrows and Enter select a row and open the detail panel", !!g && open.expanded === "true" && open.body > 0, JSON.stringify(open));
    const stillGrid = await here("panel open");
    check(name, "keyboard: focus stays on the grid while the panel is open", !!stillGrid && stillGrid.grid);
    await d.page.key("Escape"); await d.settle({ quiet: 200 });
    const closed = await d.page.eval(`document.getElementById('btnPanel').getAttribute('aria-expanded')`);
    const back = await here("panel closed");
    check(name, "keyboard: Esc closes the detail panel and focus returns to the grid", closed === "false" && !!back && back.grid, JSON.stringify([closed, back?.tag, back?.cls]));

    await d.page.combo("k", 75, 2); await sleep(150);
    const pal = await d.page.eval(`({ dialog: !!document.querySelector('dialog[open]'), focus: document.activeElement?.getAttribute('role') })`);
    check(name, "keyboard: Ctrl+K opens the palette with focus in its field", pal.dialog && pal.focus === "combobox", JSON.stringify(pal));
    await d.type("materiales"); await sleep(100);
    await d.page.key("Enter"); await sleep(250); await d.settle({ quiet: 250 });
    const jumped = await d.page.eval(`({ hash: location.hash, view: document.querySelector('#main [data-view]')?.dataset.view, dialog: !!document.querySelector('dialog[open]') })`);
    check(name, "keyboard: the palette jumps to the chosen view and closes", jumped.hash.startsWith("#/materials") && jumped.view === "materials" && !jumped.dialog, JSON.stringify(jumped));
    await sleep(60);
    await here("after palette");
    await d.page.tab(true); await here("shift+tab");
    check(name, "keyboard: focus was never lost to <body> along the whole path", lost.length === 0, lost.slice(0, 5).join(", "));
  }

  // ---- 900 px: no overflow in any view; the detail panel overlays and Esc closes it
  await d.page.send("Emulation.setDeviceMetricsOverride", { width: 900, height: 800, deviceScaleFactor: 1, mobile: false });
  const over = [];
  for (const view of views) { await openView(d, view); const o = await d.page.eval(AUDIT); const w = await d.page.eval(`document.getElementById('main').scrollWidth - document.getElementById('main').clientWidth`); if (o.overflow > 0 || w > 1) over.push(`${view}:${o.overflow}/${w}`); }
  check(name, "a11y: every view is usable at 900 px (no horizontal scroll)", over.length === 0, over.join(" | "));
  await openView(d, "catalog");
  await d.page.eval(`document.getElementById('btnPanel').focus()`);
  await d.page.key("Enter"); await d.settle({ quiet: 200 });
  const ov = await d.page.eval(`(() => { const e = document.getElementById('detail'); const r = e.getBoundingClientRect(); return { pos: getComputedStyle(e).position, visible: r.width > 0, right: Math.round(r.right), iw: innerWidth }; })()`);
  check(name, "a11y: at 900 px the detail panel is an overlay inside the window", ov.pos === "fixed" && ov.visible && ov.right <= ov.iw, JSON.stringify(ov));
  await shot(d, `${name}-app-catalog-w900.png`);
  await d.page.key("Escape"); await d.settle({ quiet: 200 });
  const after = await d.page.eval(`({ expanded: document.getElementById('btnPanel').getAttribute('aria-expanded'), w: document.getElementById('detail').getBoundingClientRect().width, focus: document.activeElement?.id })`);
  check(name, "a11y: Esc closes the overlay panel and focus returns to Detalle", after.expanded === "false" && after.w === 0 && after.focus === "btnPanel", JSON.stringify(after));
  await d.page.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  check(name, "a11y: walkthrough ran without JS errors", d.errors.length === mark0, d.errors.slice(mark0).join(" | "));
}
