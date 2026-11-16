// Walkthrough of the second batch of views of the new UI (/): Contexto, Fichas, Montaje,
// Trabajos (with a followed job across a reload), the 3D viewer and the wizard.
import { sleep } from "./cdp.mjs";

const q = JSON.stringify;
const text = (d, sel) => d.page.eval(`document.querySelector(${q(sel)})?.innerText ?? ''`);
const count = (d, sel) => d.page.eval(`document.querySelectorAll(${q(sel)}).length`);
async function go(d, port, hash, view) {
  await d.goto(`http://127.0.0.1:${port}/${hash}`);
  const ok = await d.waitFor(`() => document.querySelector('#main [data-view=${q(view)}][data-ready]')`);
  await d.settle({ quiet: 250 });
  return ok;
}

// A job that runs ~9 s, whose clock lives in sessionStorage so it survives a reload.
const MOCK = `(() => {
  const of = window.fetch.bind(window);
  window.fetch = (u, ...r) => {
    const url = String(u);
    if (!/\\/api\\/jobs(\\/\\d+)?(\\?|$)/.test(url)) return of(u, ...r);
    const t0 = Number(sessionStorage.kmock || (sessionStorage.kmock = Date.now())), age = Date.now() - t0, run = age < 9000;
    const log = Array.from({ length: Math.min(80, Math.floor(age / 30)) }, (_, i) => "linea " + i);
    const body = url.endsWith("/api/jobs") ? { jobs: [{ id: 99, label: "montar puerta (simulado)", state: run ? "running" : "done", kind: "montar", started: 1700000000 }] }
      : { asked: "montar puerta (simulado)", status: run ? "running" : "done", log, spec: null, kind: "montar" };
    return Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } }));
  };
})()`;

export async function walkNewViews({ d, name, port, cat, check, note, shot }) {
  const mark = d.errors.length;
  // ---- Contexto
  check(name, "context: view renders with text, layers and the talk panel", await go(d, port, "#/context", "context") && (await count(d, "#xText")) === 1 && (await count(d, "#xLayers a[data-layer]")) >= 2 && (await count(d, "#xTalk .msgs")) === 1);
  await shot(d, `${name}-app-context.png`);
  const add = await count(d, '[data-x="add"]');
  if (add) {
    await d.clickAndSettle(`() => document.querySelector('[data-x="add"]')`);
    check(name, "context: adding a claim opens a dialog (not prompt) that Esc closes without writing", (await count(d, "dialog[open]")) === 1);
    await d.page.key("Escape"); await sleep(150);
    check(name, "context: the dialog is gone after Esc", (await count(d, "dialog")) === 0);
  }
  const lay = await d.page.eval(`document.querySelectorAll('#xLayers a[data-layer]:not([data-layer=""])')[0]?.dataset.layer ?? null`);
  if (lay) {
    await d.clickAndSettle(`() => document.querySelector('#xLayers a[data-layer]:not([data-layer=""])')`);
    check(name, "context: choosing a layer keeps it in the URL", (await d.page.eval("location.hash")).includes(encodeURIComponent(lay)));
  }
  check(name, "context: no JS errors", d.errors.length === mark, d.errors.slice(mark).join(" | "));

  // ---- Fichas and Montaje
  const sp = cat.specs[0];
  if (sp) {
    const id = sp.asset.id;
    check(name, "spec: the list shows every spec", await go(d, port, "#/spec", "spec") && (await count(d, "table.tbl tbody tr")) === cat.specs.length);
    check(name, "spec: the ask form without text answers with a toast", await (async () => { await d.clickAndSettle(`() => document.querySelector('#sAsk button')`); return !!(await d.page.eval(`document.querySelector('.toast')`)); })());
    check(name, "spec: the detail renders parts and findings", await go(d, port, `#/spec/${encodeURIComponent(id)}`, "spec") && (await count(d, "tr[data-part]")) === sp.parts.length);
    await shot(d, `${name}-app-spec.png`);
    if (sp.parts.length) {
      await d.clickAndSettle(`() => document.querySelector('tr[data-part]')`);
      check(name, "spec: selecting a part fills the panel", /pieza de la ficha/i.test(await text(d, "#detail")) , (await text(d, "#detail")).slice(0, 60));
      const v = await count(d, "#detail [data-validate]");
      if (v) {
        await d.clickAndSettle(`() => document.querySelector('#detail [data-validate]')`);
        check(name, "spec: validating asks first (dialog) and Esc leaves everything as it was", (await count(d, "dialog[open]")) === 1);
        await d.page.key("Escape"); await sleep(150);
      }
    }
    check(name, "build: the view renders its actions and the review area", await go(d, port, `#/build/${encodeURIComponent(id)}`, "build") && (await count(d, "[data-job]")) === 3 && (await count(d, "#bReview")) === ((await text(d, "#main")).includes("Aún no está montado") ? 0 : 1));
    await sleep(600);
    await shot(d, `${name}-app-build.png`);
  } else check(name, "spec: examples-like project without specs shows the empty state", await go(d, port, "#/spec", "spec") && /Todavía no hay fichas/.test(await text(d, "#main")));
  check(name, "spec/build: no JS errors", d.errors.length === mark, d.errors.slice(mark).join(" | "));

  // ---- numbers and the catalog header
  if (cat.pieces.length) {
    await go(d, port, "#/catalog", "catalog");
    const rowsText = await d.page.eval(`[...document.querySelectorAll('.vrow .vc.r')].map(r => r.innerText).join(' ')`);
    check(name, "numbers: no 4+ digit number is left ungrouped in the catalog", !/(?<![.,\d])\d{4,}(?![.,\d])/.test(rowsText), rowsText.slice(0, 100));
    const clip = await d.page.eval(`[...document.querySelectorAll('.vh > div, .vh .hbtn')].filter(c => c.scrollWidth > c.clientWidth + 1).map(c => c.textContent.trim() + ':' + c.scrollWidth + '>' + c.clientWidth)`);
    check(name, "catalog: header columns do not overlap (no clipped header cell)", clip.length === 0, clip.join(" | "));
  }

  // ---- 3D viewer, opened and closed 20 times
  const withPreview = cat.pieces.find(p => p.preview);
  if (withPreview) {
    await go(d, port, "#/catalog", "catalog");
    const r = await d.page.eval(`(async () => {
      const m = await import("/ui/app/views/view3d.js");
      let shown = 0;
      for (let i = 0; i < 20; i++) {
        const p = m.openViewerDialog(document, { title: "prueba", path: ${q(withPreview.preview)}, materials: [] });
        for (let t = 0; t < 80 && !document.querySelector("dialog canvas") && !/No se pudo/.test(document.querySelector("dialog")?.innerText ?? ""); t++) await new Promise(r => setTimeout(r, 50));
        if (document.querySelector("dialog canvas")) shown++;
        document.querySelector("dialog").close(); await p;
      }
      const c = document.createElement("canvas"), gl = c.getContext("webgl2") || c.getContext("webgl");
      return { shown, created: m.stats.created, disposed: m.stats.disposed, dialogs: document.querySelectorAll("dialog").length, canvases: document.querySelectorAll("canvas").length, freshContext: !!gl && !gl.isContextLost() };
    })()`);
    check(name, "3d: opened and closed 20 times, every viewer drawn and every renderer disposed", (r.shown === 20 || name === "synthetic") && r.created === 20 && r.disposed === 20 && r.dialogs === 0 && r.canvases === 0, JSON.stringify(r));
    check(name, "3d: a fresh WebGL context is still available afterwards", r.freshContext, JSON.stringify(r));
    await d.page.eval(`location.hash = ${q("#/catalog/" + encodeURIComponent(withPreview.kit) + "/" + encodeURIComponent(withPreview.key.startsWith(withPreview.kit + "/") ? withPreview.key.slice(withPreview.kit.length + 1) : withPreview.key))}`);
    await d.settle({ quiet: 300 });
    check(name, "3d: the piece detail has its Ver en 3D button", (await count(d, "#detail [data-view3d]")) === 1);
  } else note(name, "no piece with a preview: the 3D open/close check was skipped");

  // ---- Trabajos: empty, then a followed job across a reload
  check(name, "jobs: with no jobs the view says so and offers where jobs come from", await go(d, port, "#/jobs", "jobs") && /No hay trabajos/.test(await text(d, "#jDetail")));
  await shot(d, `${name}-app-jobs-empty.png`);
  if (name === "examples") {
    await d.page.eval("sessionStorage.removeItem('kmock')");
    const mockId = (await d.page.send("Page.addScriptToEvaluateOnNewDocument", { source: MOCK })).identifier;
    try {
      await d.goto(`http://127.0.0.1:${port}/?mock=1#/jobs/99`); // a new document, so the mock script runs
      check(name, "jobs: following a job shows its state and log", await d.waitFor(`() => document.querySelector('#jLog') && /en marcha/.test(document.querySelector('#jState')?.textContent ?? '')`));
      await sleep(1500);
      check(name, "jobs: the status bar pill links to Trabajos while it runs", await d.page.eval(`!!document.querySelector('#status a[href^="#/jobs"]')`));
      await d.page.eval(`document.getElementById('jLog').scrollTop = 0`); await sleep(2200);
      const st = await d.page.eval(`({ top: document.getElementById('jLog').scrollTop, tail: !document.getElementById('jTail').hidden, lines: document.querySelectorAll('#jLog div').length })`);
      check(name, "jobs: scrolled up, the log does not jump and offers Ir al final", st.top < 5 && st.tail && st.lines > 30, JSON.stringify(st));
      await d.clickAndSettle(`() => document.getElementById('jTail')`);
      check(name, "jobs: Ir al final goes to the end", await d.page.eval(`(l => l.scrollHeight - l.scrollTop - l.clientHeight < 30)(document.getElementById('jLog'))`));
      await shot(d, `${name}-app-jobs-running.png`);
      await d.page.eval("location.reload()"); await sleep(300);
      check(name, "jobs: after a reload the same job is followed again with its log", await d.waitFor(`() => document.querySelectorAll('#jLog div').length > 5 && /simulado/.test(document.querySelector('#jTitle')?.textContent ?? '')`));
      check(name, "jobs: it ends in 'hecho' with what to do next", await d.waitFor(`() => /hecho/.test(document.querySelector('#jState')?.textContent ?? '') && document.querySelector('#jNext .callout')`, 20000));
      await shot(d, `${name}-app-jobs-done.png`);
    } finally {
      await d.page.send("Page.removeScriptToEvaluateOnNewDocument", { identifier: mockId });
      await d.page.eval("sessionStorage.removeItem('kmock')").catch(() => {});
    }
  }
  check(name, "new views: no JS errors", d.errors.length === mark, d.errors.slice(mark).join(" | "));
  await go(d, port, "#/home", "home"); // leave a clean home page for the checks that follow
  await d.clickAndSettle(`() => document.querySelector('#main h1')`); // a real click gives the page focus back
}

export async function walkAppWizard({ Driver, chrome, name, port, check, shot }) {
  console.log(`\n== ${name}/ (port ${port})`);
  const d = await Driver.open(chrome.port, `${name}/`);
  try {
    await d.goto(`http://127.0.0.1:${port}/`);
    const ok = await d.waitFor(`() => document.querySelector('#main [data-view="wizard"][data-ready]')`);
    await d.settle({ quiet: 300 });
    check(name, "app wizard: / opens the wizard on a folder without kiln.toml", ok);
    check(name, "app wizard: five steps, the first one current", (await count(d, ".steps li")) === 5 && /1 · proyecto/i.test(await text(d, ".steps li[aria-current=step]")));
    check(name, "app wizard: name field and the talk panel are there", (await count(d, "#wzName")) === 1 && (await count(d, "#wzTalk .msgs")) === 1 && (await count(d, "#tText")) === 1);
    check(name, "app wizard: the navigation is out of the way", !(await d.visible("nav.side")));
    const heads = await d.page.eval(`(() => { const hs = [...document.querySelectorAll('h1,h2,h3')].filter(h => h.getClientRects().length); let prev = 0, bad = []; for (const h of hs) { const n = Number(h.tagName[1]); if (prev && n > prev + 1) bad.push(h.tagName); prev = n; } return { h1: hs.filter(h => h.tagName === 'H1').length, first: hs[0]?.tagName, bad }; })()`);
    check(name, "a11y: the wizard has exactly one h1 and no skipped heading level", heads.h1 === 1 && heads.first === "H1" && heads.bad.length === 0, JSON.stringify(heads));
    await shot(d, "app-wizard-1.png");
    await d.clickAndSettle(`() => document.getElementById('wzNext')`);
    check(name, "app wizard: Seguir without a name says what is missing and stays", (await count(d, ".toast")) >= 1 && (await count(d, "#wzName")) === 1);
    await d.page.eval(`document.getElementById('wzName').focus()`); await d.type("Puerto Niebla"); await d.page.key("Enter"); await d.settle();
    check(name, "app wizard: with a name it goes to step 2 (Lo básico) with its questions", await d.waitFor(`() => document.querySelector('.wizard[data-step="2"] .qcard')`) && (await count(d, ".qcard")) >= 3);
    await shot(d, "app-wizard-2.png");
    await d.clickAndSettle(`() => document.getElementById('wzBack')`);
    check(name, "app wizard: Atrás returns to step 1 keeping the name", (await d.page.eval(`document.getElementById('wzName')?.value`)) === "Puerto Niebla");
    check(name, "app wizard: no JS errors", d.errors.length === 0, d.errors.join(" | "));
  } finally { await d.close(); }
}
