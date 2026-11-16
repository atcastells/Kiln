// Montaje: build and review rounds of an asset, materials by zone, search and
// import of external models, paint. Route: #/build[/id]?part=
import { html, safeUrl } from "../html.js";
import { format } from "../router.js";
import { fmtN, fmtSize, plural, dl, postJSON, worstOfList, sevBadge } from "./common.js";
import { launchJob } from "./jobs.js";
import { createViewer } from "./view3d.js";
import { specFindings } from "./spec.js";

export const SITES = { polyhaven: "Poly Haven", polypizza: "Poly Pizza", sketchfab: "Sketchfab" };
const SHEETS = [["1-turn.png", "Giro"], ["2-elevations.png", "Alzados"], ["3-parts.png", "Piezas"], ["4-silhouette.png", "Silueta"]];
const CRIT = { reads: "se entiende", proportion: "proporción", construction: "construcción", style: "estilo", material: "material", interest: "interés" };
const SEV = { blocker: ["grave", "error"], major: ["importante", "warn"], minor: ["menor", "info"] };
export const partOf = n => n.slice(0, n.indexOf("/"));
export const zoneOf = n => n.slice(n.indexOf("/") + 1);
export const slug = t => String(t ?? "").trim().toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "");
export const dirOf = p => p.replace(/[^/\\]+$/, "");

// What the spec says a zone wears: the variant, the part's zone, the part, the catalog; "" is the model's own.
export function chosen(sp, cat, name, variant) {
  const p = sp.parts.find(x => x.id === partOf(name)), from = p && cat.pieces.find(c => c.key === p.source.from);
  const v = variant && (sp.variants ?? []).find(x => x.id === variant);
  const pal = k => (k && cat.materials.some(m => m.key === k) ? k : "");
  return pal(v?.materials?.[name]) || pal(p?.materials?.[zoneOf(name)]) || pal(p?.material) || pal(from?.bindings?.[zoneOf(name)]) || "";
}

// A search candidate. Links to the sites go through `safeUrl`: anything but http(s) is an inert `#`.
export function candidateHtml(c, i, budget) {
  const page = safeUrl(c.page_url), thumb = safeUrl(c.thumb), over = budget && c.tris > budget;
  return html`<div class="cand">${thumb !== "#" ? html`<img src="${thumb}" alt="" loading="lazy" referrerpolicy="no-referrer">` : ""}<div class="cb"><b>${c.name}</b><span>${SITES[c.site] ?? c.site} · ${c.author}</span>
    <span><span class="pill ${c.license === "CC0" ? "ok" : "info"}">${c.license}</span>${c.tris != null ? html`<span class="pill ${over ? "warn" : "ok"}">${fmtN(c.tris)} tri</span>` : ""}</span>${c.size_m ? html`<span>${fmtSize(c.size_m)}</span>` : ""}</div>
    <div class="ca">${page === "#" ? html`<a href="#" aria-disabled="true" title="El sitio no dio un enlace seguro">ver</a>` : html`<a href="${page}" target="_blank" rel="noopener noreferrer">ver</a>`}<button class="btn sm" type="button" data-import="${i}">Traer</button></div></div>`;
}

export function reviewHtml(s) {
  const url = (d, f) => "/files/" + encodeURI(d + "/" + f);
  return html`<h3 class="sec">Revisión visual · ${plural(s.rounds.length, "vuelta", "vueltas")} · ${s.model}</h3><p class="muted small">Se queda la vuelta ${s.best}: su ficha y su montaje son los que hay ahora.</p>
    ${s.rounds.slice().reverse().map(r => { const rv = r.review;
      return html`<details class="round"${r.n === s.rounds.length ? " open" : ""}><summary><b>Vuelta ${r.n}</b>${r.n === s.best ? html` <span class="pill ok">la que queda</span>` : ""} <span class="pill ${rv.verdict === "pass" ? "ok" : "warn"}">${rv.verdict === "pass" ? "aprobada" : "a revisar"}</span>${r.against ? html` <span class="muted">frente a la ${r.against}: gana la ${r.kept}</span>` : ""}</summary>
        <div class="sheets">${SHEETS.map(([f, n]) => html`<a href="${url(r.dir, f)}" target="_blank" rel="noopener" title="${n}"><img src="${url(r.dir, f)}" alt="${n}" loading="lazy"></a>`)}</div>
        ${rv.why ? html`<p class="muted">${rv.why}</p>` : ""}<p>${rv.seen}</p>
        <div class="chips">${Object.entries(rv.scores ?? {}).map(([k, v]) => html`<span class="chip">${CRIT[k] ?? k} <b>${v}</b></span>`)}${r.tris ? html`<span class="chip">${fmtN(r.tris)} tri</span>` : ""}</div>
        ${(rv.findings ?? []).map(f => html`<div class="finding"><span class="pill ${(SEV[f.severity] ?? ["", "info"])[1]}">${(SEV[f.severity] ?? [f.severity])[0]}</span> <b>${f.part}</b>: ${f.problem}${Object.keys(f.fix ?? {}).length ? html`<br><code>${Object.entries(f.fix).map(([k, v]) => k + " = " + JSON.stringify(v)).join(" · ")}</code>` : ""}${f.evidence ? html`<br><span class="muted">${f.evidence}</span>` : ""}</div>`)}
        ${(r.applied ?? []).length ? html`<h4>Aplicado para la vuelta ${r.n + 1}</h4>${r.applied.map(a => html`<div class="log">${a}</div>`)}` : ""}${(r.left ?? []).length ? html`<h4>Para ti</h4>${r.left.map(a => html`<div class="log">${a}</div>`)}` : ""}</details>`; })}`;
}

export function mount(el, ctx) {
  const { store, api } = ctx;
  const id = ctx.route.args?.[0] ?? null, partQ = ctx.route.query?.part ?? null;
  let alive = true, variant = "", viewer = null, token = 0, cands = {};
  const q = s => el.querySelector(s);
  const asset = () => store.get("catalog")?.specs.find(s => s.asset.id === id);

  function zonesBox(sp, b, cat) {
    const names = Object.keys(b.materials ?? {}); if (!names.length) return "";
    const vars = (sp.variants ?? []).map(v => v.id); if (variant && !vars.includes(variant)) vars.push(variant);
    const sel = n => { const v = (sp.variants ?? []).find(x => x.id === variant), k = variant ? (v?.materials?.[n] ?? "") : chosen(sp, cat, n, ""); return html`<option value="">${variant ? "como el asset base" : "el del modelo"}</option>${cat.materials.map(m => html`<option value="${m.key}"${m.key === k ? " selected" : ""}>${m.key}${m.name !== m.key ? " · " + m.name : ""}</option>`)}`; };
    let last = null;
    return html`<section class="card pad"><h3 class="sec">Materiales por zona</h3>
      <label class="slotrow"><span>variante</span><select class="field" id="zVar"><option value="">asset base</option>${vars.map(v => html`<option value="${v}"${v === variant ? " selected" : ""}>${v}${b.variants?.[v] ? "" : " · sin aplicar"}</option>`)}<option value="+">nueva variante…</option></select></label>
      ${names.map(n => { let head = ""; if (partOf(n) !== last) { last = partOf(n); const p = sp.parts.find(x => x.id === last), can = !variant && p && Object.keys(p.materials ?? {}).length && ["reuse", "variant"].includes(p.source.kind) && cat.pieces.some(c => c.key === p.source.from); head = html`<div class="slotrow"><b>${last}</b>${can ? html`<button class="btn sm" type="button" data-validate="${last}" title="Deja estos materiales como valor por defecto de ${p.source.from} en el catálogo">Validar</button>` : ""}</div>`; }
        return html`${head}<label class="slotrow"><span class="mono">${zoneOf(n)}</span><select class="field" data-zone="${n}">${sel(n)}</select></label>`; })}
      <div class="row"><button class="btn primary" type="button" id="doPaint">Aplicar materiales${variant ? " a la variante" : ""}</button></div><p class="muted small" id="zNote" role="status"></p></section>`;
  }
  function dress() {
    const sp = asset(), cat = store.get("catalog"), b = cat?.builds[id]; if (!sp || !b || !viewer) return;
    let pending = 0;
    for (const n of Object.keys(b.materials ?? {})) {
      const k = chosen(sp, cat, n, variant), worn = variant && b.variants?.[variant] ? null : b.materials[n] || "";
      if (worn === null || k !== worn) { viewer.wear(n, k); if (worn !== null) pending++; }
    }
    const note = q("#zNote");
    if (note) note.textContent = variant && !b.variants?.[variant] ? "Esta variante aún no tiene su GLB: «Aplicar materiales» lo crea." : pending ? `${plural(pending, "zona cambiada", "zonas cambiadas")} en la ficha que el GLB aún no lleva: «Aplicar materiales» lo pone al día.` : "";
  }
  async function showViewer() {
    const cat = store.get("catalog"), b = cat?.builds[id], box = q("#b3d"), my = ++token;
    viewer?.dispose(); viewer = null;
    if (!b || !box) return;
    const v = await createViewer(box, (variant && b.variants?.[variant]) || b.glb, { materials: cat.materials });
    if (!alive || my !== token) { v.dispose(); return; }
    viewer = v; dress();
  }
  async function showReview(b) {
    const box = q("#bReview"); if (!box || !b) return;
    try {
      const r = await fetch("/files/" + encodeURI(dirOf(b.glb) + "review/review.json") + "?t=" + Date.now());
      if (!r.ok) { box.innerHTML = html`<h3 class="sec">Revisión visual</h3><p class="muted">Aún no hay revisión. «Montar y revisar» la hace.</p>`.toString(); return; }
      const s = await r.json();
      if (alive && box.isConnected) box.innerHTML = reviewHtml(s).toString();
    } catch { if (alive && box.isConnected) box.innerHTML = html`<h3 class="sec">Revisión visual</h3><p class="muted">No se pudo leer la revisión. Pulsa Releer el proyecto para volver a intentarlo.</p>`.toString(); }
  }

  function draw() {
    const cat = store.get("catalog"); if (!cat) { el.innerHTML = ""; return; }
    const sp = id && asset();
    if (!sp) {
      el.innerHTML = html`<div class="page" data-view="build" data-ready="1"><div class="pagehead"><div><h1>Montaje</h1><div class="sub">Monta y revisa los assets con ficha</div></div></div>
        ${id ? html`<div class="callout" role="alert"><b>No hay ficha «${id}»</b><p>Elige un asset de la lista.</p></div>` : ""}
        ${cat.specs.length ? html`<div class="card"><table class="tbl"><thead><tr><th>Asset</th><th>Montaje</th><th class="r">Triángulos</th></tr></thead><tbody>${cat.specs.map(s => html`<tr><td><a href="${format({ view: "build", args: [s.asset.id] })}">${s.asset.name}</a></td><td>${cat.builds[s.asset.id] ? "montado" : html`<span class="muted">sin montar</span>`}</td><td class="r">${fmtN(cat.builds[s.asset.id]?.tris)}</td></tr>`)}</tbody></table></div>` : html`<div class="empty"><h3>Todavía no hay assets con ficha</h3><p>Pide un asset desde Fichas y vuelve aquí para montarlo.</p><a class="btn primary" href="#/spec">Ir a Fichas</a></div>`}</div>`.toString();
      return;
    }
    const b = cat.builds[id], a = sp.asset, todo = (b?.items ?? []).filter(i => i.todo), running = !!store.get("job");
    const sites = sp.parts.filter(p => p.source.kind === "search" || SITES[p.source.from?.split("/")[0]]);
    el.innerHTML = html`<div class="page" data-view="build" data-ready="1"><div class="pagehead"><div><h1>Montaje · ${a.name}</h1><div class="sub">${b ? `montado ${b.built_at}` : "sin montar"} · ${sevBadge(worstOfList(specFindings(cat, id)))}</div></div><a class="btn" href="${format({ view: "spec", args: [id] })}">Ver la ficha</a></div>
      <section class="card pad"><div class="row">
        <button class="btn" type="button" data-job="layout"${running ? ' disabled title="Hay un trabajo en curso"' : ""}>${sp.parts.some(p => (p.at ?? []).length) ? "Recolocar las piezas" : "Colocar las piezas"}</button>
        <button class="btn primary" type="button" data-job="build"${running ? ' disabled title="Hay un trabajo en curso"' : ""}>Montar en Blender</button>
        <button class="btn" type="button" data-job="review"${running ? ' disabled title="Hay un trabajo en curso"' : ""}>Montar y revisar</button></div>
        <p class="muted small">${running ? "Hay un trabajo en marcha: espera a que termine (míralo en Trabajos)." : "Montar necesita Blender; Colocar y Revisar necesitan al agente. Cada uno se sigue en Trabajos."}</p></section>
      ${b ? html`<div class="split2"><section class="card pad"><h3 class="sec">Modelo</h3><div class="view3d" id="b3d" aria-label="Vista 3D del montaje"></div></section>
        <section class="card pad"><h3 class="sec">Montaje</h3>${dl([["Fichero", b.glb], ["Medida", fmtSize(b.size_m)], ["Triángulos", a.budget_tris ? `${fmtN(b.tris)} de ${fmtN(a.budget_tris)}` : fmtN(b.tris)], ["Montado", b.built_at]])}
          <h3 class="sec">Por terminar a mano <span class="n">${todo.length}</span></h3>${todo.length ? todo.map(i => html`<div class="finding"><b>${i.name}</b>: ${i.todo}</div>`) : html`<p class="muted">Nada.</p>`}</section></div>`
        : html`<div class="empty small"><h3>Aún no está montado</h3><p>${sp.parts.every(p => (p.at ?? []).length >= (p.count || 1)) ? "Las piezas están colocadas: monta en Blender." : "Faltan posiciones: pide colocar las piezas primero."}</p></div>`}
      ${b ? zonesBox(sp, b, cat) : ""}
      ${sites.length ? html`<section class="card pad"><h3 class="sec">Modelos de sitios de assets</h3><p class="muted small">Solo salen las licencias que admite el proyecto; nada se baja hasta que pulses Traer.</p>
        ${sites.map(p => html`<div class="sitepart" data-sp="${p.id}"><div class="row"><b>${p.what}</b><span class="muted small">${p.source.kind === "search" ? `busca «${p.source.query || p.what}»` : `traída de ${SITES[p.source.from.split("/")[0]]}`}</span><button class="btn sm" type="button" data-search="${p.id}"${partQ === p.id ? " data-auto" : ""}>${p.source.kind === "search" ? "Buscar" : "Buscar otra"}</button></div><div id="c-${p.id}" aria-live="polite"></div></div>`)}</section>` : ""}
      <section class="card pad" id="bReview"></section></div>`.toString();
    Object.keys(cands).forEach(pid => paintCands(pid));
    showViewer(); showReview(b);
    const auto = q("[data-auto]"); if (auto) auto.scrollIntoView({ block: "center" });
  }
  function paintCands(pid) {
    const box = q(`#c-${CSS.escape(pid)}`), r = cands[pid], sp = asset(); if (!box || !r) return;
    if (r.loading) { box.innerHTML = html`<p class="muted">Preguntando a los sitios…</p>`.toString(); return; }
    const budget = sp.asset.budget_tris;
    box.innerHTML = html`<p class="muted small">${plural(r.candidates.length, "candidato", "candidatos")} para «${r.query}»${r.size_m ? " · la pieza mide " + fmtSize(r.size_m) : ""}${budget ? " · presupuesto " + fmtN(budget) + " tri" : ""}</p>${(r.notes ?? []).map(n => html`<div class="finding"><span class="pill warn">aviso</span> ${n}</div>`)}<div class="cands">${r.candidates.map((c, i) => candidateHtml(c, i, budget))}</div>`.toString();
  }

  async function launch(btn) {
    const sp = asset(), k = btn.dataset.job;
    const spec = { layout: ["/api/ask", { layout: sp.file }, "colocar las piezas"], build: ["/api/build", { asset: id }, "montar en Blender"], review: ["/api/review", { asset: id }, "montar y revisar"] }[k];
    await ctx.busy(btn, "Lanzando el trabajo…", () => launchJob(ctx, { path: spec[0], body: spec[1], verb: spec[2], done: () => alive && draw() }));
  }
  async function search(btn) {
    const pid = btn.dataset.search; cands[pid] = { loading: true }; paintCands(pid);
    try { const r = await ctx.busy(btn, "Preguntando a los sitios…", () => postJSON(api, "/api/search", { asset: id, part: pid })); cands[pid] = r; }
    catch (e) { delete cands[pid]; q(`#c-${CSS.escape(pid)}`).innerHTML = ""; ctx.toasts.error("No se pudo buscar", `${e.message} Comprueba la conexión y vuelve a pulsar Buscar.`); return; }
    paintCands(pid);
  }
  async function importCand(btn) {
    const pid = btn.closest(".sitepart").dataset.sp, c = cands[pid].candidates[+btn.dataset.import];
    await ctx.busy(btn, "Trayendo el modelo…", () => launchJob(ctx, { path: "/api/import", body: { asset: id, part: pid, site: c.site, id: c.id }, verb: `traer «${c.name}»`, done: () => alive && (delete cands[pid], draw()) }));
  }
  async function zoneChange(s) {
    const n = s.dataset.zone;
    try { await ctx.busy(s, "Guardando el material de la zona…", () => postJSON(api, "/api/material", { asset: id, part: partOf(n), zone: zoneOf(n), material: s.value, variant })); await ctx.reload(); }
    catch (e) { ctx.toasts.error("No se pudo cambiar el material", `${e.message} Elige otro material de la paleta.`); }
  }
  async function validate(btn) {
    try { await ctx.busy(btn, "Validando…", () => postJSON(api, "/api/validate", { asset: id, part: btn.dataset.validate })); await ctx.reload(); ctx.toasts.ok("Materiales validados", "Son los de la pieza en el catálogo."); }
    catch (e) { ctx.toasts.error("No se pudo validar", `${e.message} Solo se valida una pieza que viene del catálogo.`); }
  }
  const onClick = async e => {
    const j = e.target.closest("[data-job]"), s = e.target.closest("[data-search]"), i = e.target.closest("[data-import]"), v = e.target.closest("[data-validate]");
    if (j) return launch(j);
    if (s) return search(s);
    if (i) return importCand(i);
    if (v) return validate(v);
    if (e.target.closest("#doPaint")) await launchJob(ctx, { path: "/api/paint", body: { asset: id, variant }, verb: "aplicar los materiales", done: () => alive && draw() });
  };
  const onChange = async e => {
    if (e.target.id === "zVar") {
      let v = e.target.value;
      if (v === "+") { const t = await ctx.askText({ title: "Nueva variante", text: "Una variante es el mismo asset con otros materiales.", label: "Nombre corto (letras, números, guiones)", confirm: "Crear", validate: x => (slug(x) ? "" : "Usa letras, números o guiones.") }); v = t == null ? variant : slug(t); }
      variant = v; draw();
    } else if (e.target.matches?.("select[data-zone]")) zoneChange(e.target);
  };
  el.addEventListener("click", onClick); el.addEventListener("change", onChange);
  draw();
  const off = store.subscribe("catalog", draw);
  return () => { alive = false; token++; off(); viewer?.dispose(); viewer = null; el.removeEventListener("click", onClick); el.removeEventListener("change", onChange); el.innerHTML = ""; };
}
