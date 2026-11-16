import { fmtN } from "../format.js";
// Materiales: every material of the project in a virtual table (no cut),
// with filter, sort in the URL and a panel with where it is used.
// Route: #/materials[/key]?q=&sort=
import { html } from "../html.js";
import { createList, nextSort, sortRows, filterRows } from "../ui/list.js";
import { pieceRoute } from "../search.js";
import { SEV_RANK, SEV_LABEL, plural, norm, dl, findingsBlock, replaceRoute, parseSort, formatSort, highlight } from "./common.js";

export const SORT_KEYS = ["key", "name", "source", "file", "uses", "sev"];

// material key -> [{ piece, slot, bound }]: a slot uses the material it is bound
// to, or the one with the slot's own name.
export function materialUsage(cat) {
  const keys = new Set((cat.materials ?? []).map(m => m.key));
  const use = new Map();
  for (const p of cat.pieces) for (const slot of p.material_slots ?? []) {
    const bound = p.bindings?.[slot], key = bound || slot;
    if (!keys.has(key)) continue;
    const list = use.get(key);
    const entry = { piece: p, slot, bound: !!bound };
    if (list) list.push(entry); else use.set(key, [entry]);
  }
  return use;
}

export function materialRows(cat, d) {
  const use = materialUsage(cat);
  return (cat.materials ?? []).map(m => ({ m, uses: use.get(m.key)?.length ?? 0, sev: d.worst.get(m.key) ?? null, hay: norm(`${m.key} ${m.name} ${m.file} ${m.source}`) }));
}
export const materialFilter = q => { const n = norm(q).trim(); return n ? r => r.hay.includes(n) : null; };
const sourceText = s => (s === "table" ? "tabla" : "textura");

export function mount(el, ctx) {
  let restoring = false;
  const { store } = ctx;
  const q0 = ctx.route.query ?? {};
  const state = { q: q0.q ?? "", key: ctx.route.args?.[0] ?? null, sort: parseSort(q0.sort, SORT_KEYS) };
  let cat, d, rows, usage, list = null, first = true;

  const routeNow = () => ({ view: "materials", args: state.key ? [state.key] : [], query: { q: state.q || null, sort: formatSort(state.sort) } });
  const syncUrl = () => replaceRoute(routeNow());

  function body(m) {
    const detail = Object.entries(m.detail ?? {}).map(([k, v]) => [k, JSON.stringify(v)]);
    const uses = usage.get(m.key) ?? [];
    return html`<section><h3 class="sec">Datos</h3>${dl([["Nombre", m.name], ["Origen", sourceText(m.source)], ["Fichero", m.file], ...detail])}</section>
      <section><h3 class="sec">Ranuras que la usan <span class="n">${uses.length}</span></h3>${uses.length
        ? html`<ul class="minilist">${uses.slice(0, 50).map(u => html`<li><a href="#/catalog/${encodeURIComponent(pieceRoute(u.piece).args[0])}/${encodeURIComponent(pieceRoute(u.piece).args[1])}">${u.piece.key}</a><span class="muted">${u.slot} · ${u.bound ? "vinculada" : "coincide con la paleta"}</span></li>`)}</ul>${uses.length > 50 ? html`<p class="muted small">y ${uses.length - 50} más; filtra el catálogo por «${m.key}» para verlas todas.</p>` : ""}`
        : html`<p class="muted">Ninguna pieza la usa todavía. Vincúlala desde la ranura de una pieza en el catálogo.</p>`}
        ${uses.length ? html`<a class="btn sm" href="#/catalog?q=${encodeURIComponent(m.key)}">Ver las piezas en el catálogo</a>` : ""}</section>
      ${findingsBlock(d.findingsBySubject.get(m.key), "Hallazgos de este material")}`;
  }
  function show(m, open = true) {
    state.key = m.key;
    const sel = { kind: `Material · ${sourceText(m.source)}`, title: m.key, body: body(m) };
    if (open) ctx.detail.show(sel); else store.set("selection", sel);
  }
  const columns = [
    { key: "key", label: "Clave", width: "minmax(0,1.3fr)", mono: true, value: r => r.m.key, render: r => highlight(r.m.key, state.q) },
    { key: "name", label: "Nombre", width: "minmax(0,1.3fr)", value: r => r.m.name, render: r => highlight(r.m.name, state.q) },
    { key: "source", label: "Origen", width: "90px", value: r => r.m.source, render: r => sourceText(r.m.source) },
    { key: "file", label: "Fichero", width: "minmax(0,1.6fr)", mono: true, value: r => r.m.file, render: r => r.m.file },
    { key: "uses", label: "Usos", width: "70px", align: "r", value: r => r.uses, render: r => fmtN(r.uses) },
    { key: "sev", label: "Estado", width: "110px", value: r => 3 - (SEV_RANK[r.sev] ?? 0), render: r => (r.sev ? html`<span class="sev ${r.sev}"><span class="dot ${r.sev}" aria-hidden="true"></span>${SEV_LABEL[r.sev]}</span>` : html`<span class="muted">—</span>`) },
  ];
  const paintCount = () => {
    const n = list.count();
    el.querySelector("#mCount").innerHTML = html`<b>${fmtN(n)}</b> ${n === 1 ? "material" : "materiales"}${n !== rows.length ? ` de ${fmtN(rows.length)}` : ""}`.toString();
  };

  function draw() {
    cat = store.get("catalog"); d = store.get("derived");
    if (!cat || !d) { el.innerHTML = ""; return; }
    rows = materialRows(cat, d); usage = materialUsage(cat);
    if (!rows.length) {
      el.innerHTML = html`<div class="page" data-view="materials" data-ready="1"><div class="pagehead"><div><h1>Materiales</h1><div class="sub">Paleta del proyecto</div></div></div>
        <div class="empty"><h2>Todavía no hay materiales</h2><p>Los materiales salen de las tablas y las carpetas de texturas que declara kiln.toml en [materials].</p></div></div>`.toString();
      return;
    }
    el.innerHTML = html`<div class="page fill" data-view="materials" data-ready="1">
      <div class="pagehead"><div><h1>Materiales</h1><div class="sub">${plural(rows.length, "material", "materiales")} en la paleta</div></div></div>
      <div class="pane"><div class="toolbar"><input class="field" id="mQ" type="text" aria-label="Filtrar materiales por clave, nombre o fichero" placeholder="Filtrar por clave, nombre o fichero" value="${state.q}" autocomplete="off"><span class="count" id="mCount" role="status"></span></div>
        <div class="listhost" id="mList"></div></div></div>`.toString();
    list?.destroy();
    list = createList(el.querySelector("#mList"), { columns, rows, rowId: r => r.m.key, filter: materialFilter(state.q), sort: state.sort, label: "Materiales", empty: "Ningún material coincide con el filtro.", onSelect: r => { show(r.m); if (!restoring) syncUrl(); } });
    paintCount();
    const m = state.key && cat.materials.find(x => x.key === state.key);
    if (m) {
      if (first) {
        const col = state.sort && columns.find(c => c.key === state.sort.key);
        const at = sortRows(filterRows(rows, materialFilter(state.q)), col?.value, state.sort?.dir ?? 1).findIndex(r => r.m.key === m.key);
        if (at >= 0) { restoring = true; list.select(at); restoring = false; } else show(m);
      }
      else show(m, !!store.get("panel"));
    } else { state.key = null; ctx.detail.clear(); }
    first = false;
  }
  const offs = [];
  const on = (t, ev, fn) => { t.addEventListener(ev, fn); offs.push(() => t.removeEventListener(ev, fn)); };
  on(el, "input", e => { if (e.target.id === "mQ") { state.q = e.target.value; list.setFilter(materialFilter(state.q)); paintCount(); syncUrl(); } });
  on(el, "click", e => { const h = e.target.closest(".vh button[data-key]"); if (h) { state.sort = nextSort(state.sort, h.dataset.key); syncUrl(); } });
  draw();
  const off = store.subscribe(["catalog", "derived"], draw);
  return () => { off(); offs.forEach(f => f()); list?.destroy(); ctx.detail.clear(); el.innerHTML = ""; };
}
