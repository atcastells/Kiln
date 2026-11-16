import { fmtN } from "../format.js";
// Catálogo: kits, a virtual table of pieces with filter, severity chips and
// sort kept in the URL, and a detail panel with the actions of a piece.
// Route: #/catalog[/kit][/piece]?q=&sev=&sort=
import { html } from "../html.js";
import { createList, sortRows, filterRows, nextSort } from "../ui/list.js";
import { pieceRoute } from "../search.js";
import { openViewerDialog } from "./view3d.js";
import { SEV_RANK, SEV_LABEL, SEV_SR, plural, norm, fmtSize, highlight, dl, findingsBlock, replaceRoute, parseSort, formatSort, onPlainClick, postJSON, jobLink } from "./common.js";

export const SORT_KEYS = ["name", "kit", "slots", "tris", "size", "sev"];
const SEVS = ["error", "warn", "info", "ok"];
const SPLITS = [["", "no partir"], ["loose", "por trozos sueltos"], ["texture", "por colores de la textura"]];

// One row per piece, with everything the filter needs already computed.
export function pieceRows(cat, d) {
  return cat.pieces.map(p => {
    const slots = (p.material_slots ?? []).map(s => p.bindings?.[s] || s).join(", ");
    return { p, sev: d.worst.get(p.key) ?? null, slots, hay: norm(`${p.name} ${p.key} ${p.kit} ${slots}`) };
  });
}

// The predicate for the current kit, text and severity (`ok` = no findings); null when nothing narrows.
export function catalogFilter({ kit = "", q = "", sev = "" } = {}) {
  const n = norm(q).trim();
  if (!kit && !n && !sev) return null;
  return r => (!kit || r.p.kit === kit) && (!sev || (sev === "ok" ? r.sev === null : r.sev === sev)) && (!n || r.hay.includes(n));
}

// What the URL points at: a kit, a piece (`kit/rest-of-key` or the whole key).
export function resolveArgs(args, cat) {
  const [a0, a1] = args;
  if (a0 == null) return { kit: "", piece: null };
  const kit = cat.kits.some(k => k.name === a0) ? a0 : "";
  const find = key => cat.pieces.find(p => p.key === key);
  const piece = kit ? (a1 != null ? find(a1) ?? find(`${kit}/${a1}`) ?? null : null) : find(a0) ?? null;
  return { kit: kit || piece?.kit || "", piece };
}

export const sevCounts = rows => {
  const c = { error: 0, warn: 0, info: 0, ok: 0 };
  for (const r of rows) c[r.sev ?? "ok"]++;
  return c;
};

const sortValue = { name: r => r.p.name, kit: r => r.p.kit, slots: r => r.slots || null, tris: r => r.p.tris, size: r => (r.p.size_m ? r.p.size_m[0] * r.p.size_m[1] * r.p.size_m[2] : null), sev: r => 3 - (SEV_RANK[r.sev] ?? 0) };
export const sortedRows = (rows, pred, sort) => sortRows(filterRows(rows, pred), sort && sortValue[sort.key], sort?.dir ?? 1);

export function mount(el, ctx) {
  let restoring = false;
  const { store, api } = ctx;
  const doc = el.ownerDocument, detailEl = doc.getElementById("detail");
  const q0 = ctx.route.query ?? {};
  const state = { kit: "", piece: null, q: q0.q ?? "", sev: SEVS.includes(q0.sev ?? q0.severity) ? (q0.sev ?? q0.severity) : "", sort: parseSort(q0.sort, SORT_KEYS) };
  let cat, d, rows, byKey, list = null, cols = null;
  let first = true;

  const pieceByKey = k => byKey.get(k);
  const routeNow = () => {
    const p = state.piece && pieceByKey(state.piece);
    return { view: "catalog", args: p ? pieceRoute(p).args : state.kit ? [state.kit] : [], query: { q: state.q || null, sev: state.sev || null, sort: formatSort(state.sort) } };
  };
  const syncUrl = () => replaceRoute(routeNow());

  // ---- detail panel
  function pieceBody(p) {
    const man = Object.entries(p.manifest ?? {}).filter(([k]) => k !== "credit").map(([k, v]) => [k, JSON.stringify(v)]);
    const slots = p.material_slots ?? [];
    const options = s => html`<option value="">${cat.materials.some(m => m.key === s) ? "el de la paleta con su nombre" : "sin vincular"}</option>${cat.materials.map(m => html`<option value="${m.key}"${p.bindings?.[s] === m.key ? " selected" : ""}>${m.key}${m.name !== m.key ? " · " + m.name : ""}</option>`)}`;
    return html`
      <section><h3 class="sec">Datos</h3>${dl([
        ["Kit", html`<a href="#/catalog/${encodeURIComponent(p.kit)}">${p.kit}</a>`],
        ["Fichero", p.file || "falta"], ["Formato", p.format], ["Medida", fmtSize(p.size_m)],
        ["Base", p.min ? `y = ${p.min[1].toFixed(3)} m` : "—"], ["Triángulos", fmtN(p.tris) ?? "—"], ["Texturas", p.images ?? "—"],
        ["Ranuras", slots.join(", ") || "—"], ["Licencia", (p.licenses ?? []).join(", ") || "ninguna"], ["Crédito", p.credit], ["Medida por", p.measured_by || "nadie"], ...man,
      ])}</section>
      ${p.preview ? html`<section><button class="btn sm" type="button" data-view3d>Ver en 3D</button></section>` : ""}
      ${slots.length ? html`<section><h3 class="sec">Material de cada ranura</h3><div class="slots">${slots.map(s => html`<label class="slotrow"><span class="mono">${s}</span><select class="field" data-slot="${s}">${options(s)}</select></label>`)}</div></section>` : ""}
      ${slots.length <= 1 && p.preview ? html`<section><h3 class="sec">Partir en zonas</h3><label class="slotrow"><span>al montar</span><select class="field" id="pSplit">${SPLITS.map(([v, t]) => html`<option value="${v}"${(p.split || "") === v ? " selected" : ""}>${t}</option>`)}</select></label><p class="muted small">Las zonas salen al montar: part_1, part_2… o color_1, color_2…</p></section>` : ""}
      ${findingsBlock(d.findingsBySubject.get(p.key), "Hallazgos de esta pieza")}`;
  }
  function kitBody(k) {
    return html`<section><h3 class="sec">Datos</h3>${dl([["Carpeta", k.path], ["Piezas", k.pieces], ["Licencias", (k.licenses ?? []).join(", ") || "ninguna"], ["LICENSE.md", k.has_license_file ? (k.has_credit ? "sí, con crédito" : "sí") : "no"]])}</section>
      ${findingsBlock([...(d.findingsBySubject.get(k.path) ?? []), ...(k.name !== k.path ? d.findingsBySubject.get(k.name) ?? [] : [])], "Hallazgos de este kit")}`;
  }
  const splitable = p => (p.material_slots ?? []).length <= 1 && !!p.preview;
  function pieceActions(p) {
    const see = p.preview ? [{ label: `Ver ${p.name} en 3D`, run: () => openViewerDialog(doc, { title: p.name, path: p.preview, materials: cat.materials }) }] : [];
    if (!splitable(p)) return see;
    return see.concat(SPLITS.filter(([v]) => v !== (p.split || "")).map(([v, t]) => ({ label: `Partir ${p.name}: ${t}`, run: () => doSplit(p, v, null) })));
  }
  function showPiece(p, open = true) {
    state.piece = p.key;
    const sel = { kind: `Pieza · ${p.kit}`, title: p.name, body: pieceBody(p) };
    if (open) ctx.detail.show(sel); else store.set("selection", sel);
    store.set("selectionActions", pieceActions(p));
  }
  function showKit(name) {
    const k = cat.kits.find(x => x.name === name);
    if (!k) return;
    store.set("selection", { kind: "Kit", title: k.name, body: kitBody(k) });
    store.set("selectionActions", []);
  }

  // ---- actions: busy, then a toast; a failure says what to do
  async function act(control, reason, work, ok, verb) {
    const run = async () => { await work(); await ctx.reload(); };
    try {
      await (control ? ctx.busy(control, reason, run) : run());
      ctx.toasts.ok(ok.title, ok.text, ok.action);
    } catch (e) {
      ctx.toasts.error(`No se pudo ${verb}`, `${e.message} Comprueba el valor y vuelve a intentarlo; Releer vuelve a leer el proyecto.`);
      const p = state.piece && pieceByKey(state.piece);
      if (p) showPiece(p, false);
    }
  }
  const doBind = (control, p, slot, material) => act(control, `Vinculando la ranura ${slot}…`, () => postJSON(api, "/api/bind", { piece: p.key, slot, material }), { title: material ? `Ranura ${slot} vinculada a ${material}` : `Ranura ${slot} desvinculada` }, "vincular la ranura");
  const doSplit = (p, mode, control) => act(control, "Guardando cómo se parte la pieza…", () => postJSON(api, "/api/split", { piece: p.key, mode }), { title: "Partición guardada", text: "Las zonas salen al montar la pieza." }, "guardar la partición");
  async function doGodot(btn) {
    try {
      const r = await ctx.busy(btn, "Lanzando la medición con Godot…", () => postJSON(api, "/api/godot", {}));
      await ctx.refreshJobs();
      ctx.toasts.ok("Medición con Godot en marcha", "Puedes seguirla en Trabajos.", jobLink(ctx, r.job, "Ver el trabajo"));
    } catch (e) { ctx.toasts.error("No se pudo lanzar la medición con Godot", `${e.message} Comprueba que Godot está instalado y vuelve a intentarlo.`); }
  }

  // ---- table
  const columns = showKit => [
    { key: "name", label: "Nombre", width: "minmax(100px,1.2fr)", mono: true, value: sortValue.name, render: r => highlight(r.p.name, state.q) },
    ...(showKit ? [{ key: "kit", label: "Kit", width: "minmax(56px,.6fr)", value: sortValue.kit, render: r => highlight(r.p.kit, state.q) }] : []),
    { key: "slots", label: "Material", width: "minmax(80px,.7fr)", value: sortValue.slots, render: r => r.slots || "—" },
    { key: "tris", label: "Triángulos", width: "108px", align: "r", value: sortValue.tris, render: r => (r.p.tris == null ? "—" : fmtN(r.p.tris)) },
    { key: "size", label: "Medida", width: "140px", value: sortValue.size, render: r => (r.p.size_m ? r.p.size_m.map(n => n.toFixed(2)).join(" × ") : "sin medir") },
    { key: "sev", label: "Estado", width: "84px", value: sortValue.sev, render: r => (r.sev ? html`<span class="sev ${r.sev}"><span class="dot ${r.sev}" aria-hidden="true"></span>${SEV_LABEL[r.sev]}</span>` : html`<span class="muted">—</span>`) },
  ];
  const predicate = () => catalogFilter(state);
  function buildList() {
    list?.destroy();
    cols = columns(!state.kit);
    list = createList(el.querySelector("#cList"), { columns: cols, rows, rowId: r => r.p.key, filter: predicate(), sort: state.sort, label: "Piezas", empty: "Ninguna pieza coincide con el filtro.", onSelect: r => { showPiece(r.p); if (!restoring) syncUrl(); } });
  }
  function selectByKey(key) {
    const p = pieceByKey(key);
    if (!p) return;
    const i = sortedRows(rows, predicate(), state.sort).findIndex(r => r.p.key === key);
    if (i >= 0) { restoring = true; list.select(i); restoring = false; } else showPiece(p);
  }

  // ---- the parts of the page that change with the state
  function paintKits() {
    const k = el.querySelector("#cKits");
    const sevOf = name => kitSev.get(name) ?? null;
    k.innerHTML = html`<h2>Kits</h2><a href="#/catalog" data-kit="" ${!state.kit ? html`aria-current="true"` : ""}><span class="dot none" aria-hidden="true"></span><span>Todos los kits</span><span class="cnt">${fmtN(rows.length)}</span></a><hr>${cat.kits.map(kit => html`
      <a href="#/catalog/${encodeURIComponent(kit.name)}" data-kit="${kit.name}" title="${kit.name}"${state.kit === kit.name ? html` aria-current="true"` : ""}><span class="dot ${sevOf(kit.name) ?? "none"}" aria-hidden="true"></span><span>${kit.name}${sevOf(kit.name) ? html`<span class="sr"> (${SEV_SR[sevOf(kit.name)]})</span>` : ""}</span><span class="cnt">${fmtN(d.piecesByKit.get(kit.name)?.length ?? 0)}</span></a>`)}`.toString();
  }
  function paintChips() {
    const c = sevCounts(state.kit ? rows.filter(r => r.p.kit === state.kit) : rows);
    const names = { error: "Error", warn: "Aviso", info: "Nota", ok: "Sin problemas" };
    el.querySelector("#cChips").innerHTML = SEVS.map(s => html`<button class="chip" type="button" data-sev="${s}" aria-pressed="${String(state.sev === s)}"><span class="dot ${s}" aria-hidden="true"></span>${names[s]} <b>${fmtN(c[s])}</b></button>`).join("");
  }
  function paintCount() {
    const n = list.count(), all = state.kit ? (d.piecesByKit.get(state.kit)?.length ?? 0) : rows.length;
    el.querySelector("#cCount").innerHTML = html`<b>${fmtN(n)}</b> ${n === 1 ? "pieza" : "piezas"}${n !== all ? ` de ${fmtN(all)}` : ""}`.toString();
  }
  let kitSev = new Map();
  function refresh({ relist = false } = {}) {
    if (relist) buildList(); else list.setFilter(predicate());
    paintCount(); syncUrl();
  }

  function draw() {
    cat = store.get("catalog"); d = store.get("derived");
    if (!cat || !d) { el.innerHTML = ""; return; }
    rows = pieceRows(cat, d);
    byKey = new Map(cat.pieces.map(p => [p.key, p]));
    kitSev = new Map();
    for (const r of rows) if (SEV_RANK[r.sev] > (SEV_RANK[kitSev.get(r.p.kit)] ?? 0)) kitSev.set(r.p.kit, r.sev);
    for (const k of cat.kits) for (const s of [d.worst.get(k.path), d.worst.get(k.name)]) if (SEV_RANK[s] > (SEV_RANK[kitSev.get(k.name)] ?? 0)) kitSev.set(k.name, s);
    if (first) {
      const r = resolveArgs(ctx.route.args ?? [], cat);
      state.kit = r.kit; state.piece = r.piece?.key ?? null;
      if ((ctx.route.args ?? []).length && !r.kit && !r.piece) ctx.toasts.info("No existe esa pieza ni ese kit", "Se muestra el catálogo entero.");
    }
    if (state.kit && !cat.kits.some(k => k.name === state.kit)) state.kit = "";
    if (state.piece && !byKey.has(state.piece)) state.piece = null;
    const measured = cat.findings.filter(f => f.code === "not_measured").length, stale = cat.findings.some(f => f.code === "godot_stale");
    if (!cat.kits.length && !cat.pieces.length) {
      el.innerHTML = html`<div class="page" data-view="catalog" data-ready="1"><div class="pagehead"><div><h1>Catálogo</h1><div class="sub">Piezas por kit</div></div></div>
        <div class="empty"><h3>Todavía no hay kits</h3><p>El catálogo lista las piezas de cada kit con su estado de validación. Declara un kit en kiln.toml y aparecerán aquí.</p><a class="btn primary" href="#/home">Volver al inicio</a></div></div>`.toString();
      return;
    }
    el.innerHTML = html`<div class="page fill" data-view="catalog" data-ready="1">
      <div class="pagehead"><div><h1>Catálogo</h1><div class="sub">${plural(cat.pieces.length, "pieza", "piezas")} en ${plural(cat.kits.length, "kit", "kits")}</div></div>
        <div>${measured || stale ? html`<button class="btn" type="button" id="cGodot" title="${measured ? `${plural(measured, "pieza", "piezas")} solo las abre Godot: las mide y las pasa a GLB para Blender y la vista 3D.` : "Godot tiene medidas desactualizadas."}">Medir con Godot</button>` : ""}</div></div>
      <div class="split"><nav class="sidelist card" id="cKits" aria-label="Kits"></nav>
        <div class="pane"><div class="toolbar"><input class="field" id="cQ" type="text" aria-label="Filtrar piezas por nombre, kit o material" placeholder="Filtrar por nombre, kit o material" value="${state.q}" autocomplete="off"><span class="count" id="cCount" role="status"></span></div>
          <div class="chips" id="cChips" role="group" aria-label="Filtrar por estado"></div><div class="listhost" id="cList"></div></div></div></div>`.toString();
    paintKits(); paintChips(); buildList(); paintCount();
    if (state.piece) { if (first) selectByKey(state.piece); else showPiece(pieceByKey(state.piece), !!store.get("panel")); }
    else if (state.kit) showKit(state.kit); else { ctx.detail.clear(); store.set("selectionActions", null); }
    first = false;
  }

  // ---- events, all delegated so a redraw does not stack them
  const offs = [];
  const on = (t, ev, fn) => { t.addEventListener(ev, fn); offs.push(() => t.removeEventListener(ev, fn)); };
  on(el, "input", e => { if (e.target.id === "cQ") { state.q = e.target.value; refresh(); } });
  on(el, "click", e => {
    const chip = e.target.closest("button[data-sev]");
    if (chip && el.contains(chip)) { state.sev = state.sev === chip.dataset.sev ? "" : chip.dataset.sev; paintChips(); refresh(); return; }
    if (e.target.closest("#cGodot")) { doGodot(e.target.closest("#cGodot")); return; }
    const h = e.target.closest(".vh button[data-key]");
    if (h) { state.sort = nextSort(state.sort, h.dataset.key); syncUrl(); } // createList sorted by itself; this keeps the URL in step
  });
  offs.push(onPlainClick(el, "#cKits a[data-kit]", a => {
    state.kit = a.dataset.kit;
    const p = state.piece && pieceByKey(state.piece);
    if (p && state.kit && p.kit !== state.kit) { state.piece = null; }
    paintKits(); paintChips(); refresh({ relist: true });
    if (state.piece) showPiece(pieceByKey(state.piece), false); else if (state.kit) showKit(state.kit); else { ctx.detail.clear(); store.set("selectionActions", null); }
  }));
  on(detailEl, "click", e => {
    const p = state.piece && pieceByKey(state.piece);
    if (p?.preview && e.target.closest("[data-view3d]")) openViewerDialog(doc, { title: p.name, path: p.preview, materials: cat.materials });
  });
  on(detailEl, "change", e => {
    const s = e.target.closest("select[data-slot], select#pSplit");
    const p = state.piece && pieceByKey(state.piece);
    if (!s || !p) return;
    if (s.id === "pSplit") doSplit(p, s.value, s); else doBind(s, p, s.dataset.slot, s.value);
  });
  draw();
  const off = store.subscribe(["catalog", "derived"], draw);
  return () => { off(); offs.forEach(f => f()); list?.destroy(); ctx.detail.clear(); store.set("selectionActions", null); el.innerHTML = ""; };
}
