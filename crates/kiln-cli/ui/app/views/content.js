import { fmtN } from "../format.js";
// Contenido: sets of game content -> kinds -> elements, with the findings of
// each element on its row, the findings of the set itself, and a filter.
// Route: #/content[/set][/file#id]?q=&kind=&sort=
import { html } from "../html.js";
import { createList, nextSort, sortRows, filterRows } from "../ui/list.js";
import { SEV_RANK, SEV_LABEL, SEV_SR, plural, norm, dl, findingsBlock, worstOfList, replaceRoute, parseSort, formatSort, onPlainClick, highlight } from "./common.js";

export const SORT_KEYS = ["kind", "id", "file", "sev"];

export function elementRows(set, d) {
  return set.elements.map(e => {
    const findings = d.content.byElement.get(`${set.name}:${e.file}#${e.id}`) ?? [];
    return { e, arg: `${e.file}#${e.id}`, findings, sev: worstOfList(findings), hay: norm(`${e.kind} ${e.id} ${e.file}`) };
  });
}
export function contentFilter({ q = "", kind = "" } = {}) {
  const n = norm(q).trim();
  if (!n && !kind) return null;
  return r => (!kind || r.e.kind === kind) && (!n || r.hay.includes(n));
}
// The element the URL names: `file#id`, or the id alone when it is unique in the set.
export function findElement(rows, arg) {
  if (arg == null) return null;
  return rows.find(r => r.arg === arg) ?? rows.find(r => r.e.id === arg) ?? null;
}
export const sortedElements = (rows, pred, sort, columns) => {
  const col = sort && columns.find(c => c.key === sort.key);
  return sortRows(filterRows(rows, pred), col?.value, sort?.dir ?? 1);
};

export function mount(el, ctx) {
  let restoring = false;
  const { store } = ctx;
  const doc = el.ownerDocument;
  const q0 = ctx.route.query ?? {};
  const state = { set: ctx.route.args?.[0] ?? "", arg: ctx.route.args?.[1] ?? null, q: q0.q ?? "", kind: q0.kind ?? "", sort: parseSort(q0.sort, SORT_KEYS) };
  let cat, d, set, rows, list = null, first = true;

  const routeNow = () => ({ view: "content", args: state.arg ? [state.set, state.arg] : state.set ? [state.set] : [], query: { q: state.q || null, kind: state.kind || null, sort: formatSort(state.sort) } });
  const syncUrl = () => replaceRoute(routeNow());

  const columns = [
    { key: "kind", label: "Tipo", width: "120px", value: r => r.e.kind, render: r => r.e.kind },
    { key: "id", label: "Id", width: "minmax(0,1.3fr)", mono: true, value: r => r.e.id, render: r => highlight(r.e.id, state.q) },
    { key: "file", label: "Fichero", width: "minmax(0,1.2fr)", mono: true, value: r => r.e.file, render: r => `${r.e.file}:${r.e.line}` },
    { key: "sev", label: "Comprobaciones", width: "minmax(0,1.6fr)", value: r => 3 - (SEV_RANK[r.sev] ?? 0), render: r => (r.sev
      ? html`<span class="sev ${r.sev}"><span class="dot ${r.sev}" aria-hidden="true"></span>${SEV_LABEL[r.sev]}</span> <span class="muted">${r.findings[0].message}${r.findings.length > 1 ? ` (+${r.findings.length - 1})` : ""}</span>`
      : html`<span class="muted">—</span>`) },
  ];

  function elementBody(r) {
    return html`<section><h3 class="sec">Datos</h3>${dl([["Conjunto", set.name], ["Tipo", r.e.kind], ["Id", r.e.id], ["Fichero", `${r.e.file}:${r.e.line}`]])}</section>${findingsBlock(r.findings, "Comprobaciones de este elemento")}`;
  }
  function show(r, open = true) {
    state.arg = r.arg;
    const sel = { kind: `Elemento · ${r.e.kind}`, title: r.e.id, body: elementBody(r) };
    if (open) ctx.detail.show(sel); else store.set("selection", sel);
  }
  function showSet() {
    const fs = d.content.bySet.get(set.name) ?? [];
    store.set("selection", { kind: "Conjunto de contenido", title: set.name, body: html`<section><h3 class="sec">Datos</h3>${dl([["Elementos", set.elements.length], ["Tipos", set.kinds.map(k => `${k.name} · ${k.count}`).join(", ")]])}</section>${findingsBlock(fs.filter(f => f.severity !== "info"), "Lo que hay que mirar")}` });
  }

  function paintCount() {
    const n = list.count();
    el.querySelector("#eCount").innerHTML = html`<b>${fmtN(n)}</b> ${n === 1 ? "elemento" : "elementos"}${n !== rows.length ? ` de ${fmtN(rows.length)}` : ""}`.toString();
  }
  function paintChips() {
    el.querySelector("#eChips").innerHTML = set.kinds.map(k => html`<button class="chip" type="button" data-kind="${k.name}" aria-pressed="${String(state.kind === k.name)}">${k.name} <b>${fmtN(k.count)}</b></button>`).join("");
  }
  function paintSets() {
    el.querySelector("#eSets").innerHTML = html`<h2>Conjuntos</h2>${cat.content.map(s => {
      const sev = worstOfList(d.content.bySet.get(s.name));
      return html`<a href="#/content/${encodeURIComponent(s.name)}" data-set="${s.name}" title="${s.name}"${s.name === state.set ? html` aria-current="true"` : ""}><span class="dot ${sev ?? "none"}" aria-hidden="true"></span><span>${s.name}${sev ? html`<span class="sr"> (${SEV_SR[sev]})</span>` : ""}</span><span class="cnt">${fmtN(s.elements.length)}</span></a>`;
    })}`.toString();
  }
  function paintHead() {
    const all = d.content.bySet.get(set.name) ?? [], loose = d.content.looseBySet.get(set.name) ?? [];
    const e = all.filter(f => f.severity === "error").length, w = all.filter(f => f.severity === "warn").length;
    const status = e ? html`<span class="sev error"><span class="dot error" aria-hidden="true"></span>${plural(e, "error", "errores")}</span>` : w ? html`<span class="sev warn"><span class="dot warn" aria-hidden="true"></span>${plural(w, "aviso", "avisos")}</span>` : html`<span class="sev ok"><span class="dot ok" aria-hidden="true"></span>sin problemas</span>`;
    const item = f => html`<div class="finding"><div class="fh"><span class="sev ${f.severity}"><span class="dot ${f.severity}" aria-hidden="true"></span>${SEV_LABEL[f.severity]}</span><code>${f.code}</code></div><div>${f.message}</div></div>`;
    el.querySelector("#eHead").innerHTML = html`<div class="sethead"><h2>${set.name}</h2>${status}<span class="muted">${plural(set.elements.length, "elemento", "elementos")}</span></div>
      ${!all.length ? html`<div class="calm" data-calm="1"><span class="dot ok" aria-hidden="true"></span><div><b>Sin problemas</b><span>El conjunto «${set.name}» está validado: ningún elemento tiene hallazgos.</span></div></div>` : ""}
      ${loose.length ? html`<section class="card loose"><h3 class="sec">Del conjunto <span class="n">${loose.length}</span></h3>${loose.slice(0, 5).map(item)}${loose.length > 5 ? html`<details><summary>Ver los ${loose.length - 5} restantes</summary>${loose.slice(5).map(item)}</details>` : ""}</section>` : ""}`.toString();
  }

  function buildList() {
    list?.destroy();
    list = createList(el.querySelector("#eList"), { columns, rows, rowId: r => r.arg, filter: contentFilter(state), sort: state.sort, label: `Elementos de ${set.name}`, empty: "Ningún elemento coincide con el filtro.", onSelect: r => { show(r); if (!restoring) syncUrl(); } });
  }
  function openSet(name, reset = true) {
    set = cat.content.find(s => s.name === name) ?? cat.content[0];
    state.set = set.name;
    rows = elementRows(set, d);
    if (reset) { state.arg = null; state.kind = ""; }
    paintSets(); paintHead(); paintChips(); buildList(); paintCount(); syncUrl();
  }

  function draw() {
    cat = store.get("catalog"); d = store.get("derived");
    if (!cat || !d) { el.innerHTML = ""; return; }
    if (!cat.content?.length) {
      el.innerHTML = html`<div class="page" data-view="content" data-ready="1"><div class="pagehead"><div><h1>Contenido</h1><div class="sub">Datos del juego</div></div></div>
        <div class="empty"><h3>Este proyecto no declara contenido</h3><p>Los conjuntos de contenido se declaran en kiln.toml, en [content.&lt;conjunto&gt;].</p></div></div>`.toString();
      return;
    }
    const total = cat.content.reduce((n, s) => n + s.elements.length, 0);
    el.innerHTML = html`<div class="page fill" data-view="content" data-ready="1">
      <div class="pagehead"><div><h1>Contenido</h1><div class="sub">${plural(total, "elemento", "elementos")} en ${plural(cat.content.length, "conjunto", "conjuntos")}</div></div></div>
      <div class="split"><nav class="sidelist card" id="eSets" aria-label="Conjuntos de contenido"></nav>
        <div class="pane"><div id="eHead"></div><div class="chips" id="eChips" role="group" aria-label="Filtrar por tipo"></div>
          <div class="toolbar"><input class="field" id="eQ" type="text" aria-label="Filtrar elementos por tipo, id o fichero" placeholder="Filtrar por tipo, id o fichero" value="${state.q}" autocomplete="off"><span class="count" id="eCount" role="status"></span></div>
          <div class="listhost" id="eList"></div></div></div></div>`.toString();
    const name = cat.content.some(s => s.name === state.set) ? state.set : cat.content[0].name;
    openSet(name, false);
    if (state.kind && !set.kinds.some(k => k.name === state.kind)) { state.kind = ""; paintChips(); list.setFilter(contentFilter(state)); paintCount(); }
    const r = findElement(rows, state.arg);
    if (r) {
      if (first) { const at = sortedElements(rows, contentFilter(state), state.sort, columns).findIndex(x => x === r); if (at >= 0) { restoring = true; list.select(at); restoring = false; } else show(r); }
      else show(r, !!store.get("panel"));
    } else { state.arg = null; if (first) { ctx.detail.clear(); } else showSet(); }
    first = false;
  }

  const offs = [];
  const on = (t, ev, fn) => { t.addEventListener(ev, fn); offs.push(() => t.removeEventListener(ev, fn)); };
  on(el, "input", e => { if (e.target.id === "eQ") { state.q = e.target.value; list.setFilter(contentFilter(state)); paintCount(); syncUrl(); } });
  on(el, "click", e => {
    const chip = e.target.closest("button[data-kind]");
    if (chip && el.contains(chip)) { state.kind = state.kind === chip.dataset.kind ? "" : chip.dataset.kind; paintChips(); list.setFilter(contentFilter(state)); paintCount(); syncUrl(); return; }
    const h = e.target.closest(".vh button[data-key]");
    if (h) { state.sort = nextSort(state.sort, h.dataset.key); syncUrl(); }
  });
  offs.push(onPlainClick(el, "#eSets a[data-set]", a => { openSet(a.dataset.set); showSet(); }));
  draw();
  const off = store.subscribe(["catalog", "derived"], draw);
  return () => { off(); offs.forEach(f => f()); list?.destroy(); ctx.detail.clear(); el.innerHTML = ""; };
}
