// Ficha de asset: the spec of an asset with its parts, findings and the
// validation and "pedir/desglosar" actions. Route: #/spec[/id]
import { html } from "../html.js";
import { format } from "../router.js";
import { fmtN, fmtSize, plural, dl, findingsBlock, worstOfList, sevBadge, postJSON } from "./common.js";
import { launchJob } from "./jobs.js";
import { openViewerDialog } from "./view3d.js";

export const madeOf = sp => [...new Set(sp.parts.filter(p => p.source.kind === "asset" && p.source.from).map(p => p.source.from))];
export const usedIn = (cat, id) => cat.specs.map(s => [s.asset.id, s.parts.filter(p => p.source.kind === "asset" && p.source.from === id).reduce((n, p) => n + Math.max(1, p.count || 1), 0)]).filter(([, n]) => n);
export const specFindings = (cat, id) => cat.findings.filter(f => f.subject === `asset:${id}` || f.subject.startsWith(`part:${id}/`));
export const SOURCE = { reuse: "Reutilizar", variant: "Variante", search: "Buscar", retouch: "Retocar", asset: "Asset" };
// A part whose materials can become the catalog piece's defaults.
export const canValidate = (p, cat) => Object.keys(p.materials ?? {}).length > 0 && ["reuse", "variant"].includes(p.source.kind) && cat.pieces.some(c => c.key === p.source.from);
const sourceText = p => (p.source.from ? p.source.from + (p.source.variant ? ` · ${p.source.variant}` : "") : p.source.query ? `buscar «${p.source.query}»` : "—");

export function mount(el, ctx) {
  const { store, api } = ctx;
  const id = ctx.route.args?.[0] ?? null;
  let alive = true, selected = null;
  const doc = el.ownerDocument, detailEl = doc.getElementById("detail");
  const specName = (cat, x) => cat.specs.find(s => s.asset.id === x)?.asset.name ?? x;
  const link = (cat, x) => html`<a href="${format({ view: "spec", args: [x] })}">${specName(cat, x)}</a>`;

  const askForm = html`<form class="askform" id="sAsk"><label for="sAskText" class="sec">Pedir un asset</label><div class="row"><input class="field" id="sAskText" type="text" placeholder="Pedir un asset: crear un ascensor" autocomplete="off"><button class="btn primary" type="submit">Desglosar</button></div><p class="muted small">El agente lo desglosa en piezas y escribe su ficha. Lo ves en Trabajos.</p></form>`;

  function draw() {
    const cat = store.get("catalog"); if (!cat) { el.innerHTML = ""; return; }
    const sp = id && cat.specs.find(s => s.asset.id === id);
    if (!id || !sp) {
      el.innerHTML = html`<div class="page" data-view="spec" data-ready="1"><div class="pagehead"><div><h1>Fichas</h1><div class="sub">${plural(cat.specs.length, "asset con ficha", "assets con ficha")}</div></div></div>
        ${id ? html`<div class="callout" role="alert"><b>No hay ficha «${id}»</b><p>Elige una de la lista o pídela.</p></div>` : ""}${askForm}
        ${cat.specs.length ? html`<div class="card"><table class="tbl"><thead><tr><th>Asset</th><th>Estado</th><th class="r">Piezas</th><th>Hallazgos</th><th>Montaje</th></tr></thead><tbody>${cat.specs.map(s => html`<tr><td><a href="${format({ view: "spec", args: [s.asset.id] })}">${s.asset.name}</a></td><td>${s.asset.status}</td><td class="r">${fmtN(s.parts.length)}</td><td>${sevBadge(worstOfList(specFindings(cat, s.asset.id)))}</td><td>${cat.builds[s.asset.id] ? html`<a href="${format({ view: "build", args: [s.asset.id] })}">Ver</a>` : html`<span class="muted">sin montar</span>`}</td></tr>`)}</tbody></table></div>`
          : html`<div class="empty"><h3>Todavía no hay fichas</h3><p>Pide un asset y el agente escribe su ficha: sus piezas, de dónde sale cada una y sus materiales.</p></div>`}</div>`.toString();
      return;
    }
    const a = sp.asset, fs = specFindings(cat, a.id), down = madeOf(sp), up = usedIn(cat, a.id), stale = cat.builds[a.id]?.stale ?? [];
    el.innerHTML = html`<div class="page" data-view="spec" data-ready="1"><div class="pagehead"><div><h1>${a.name}</h1><div class="sub">${a.status} · «${a.asked}»</div></div>
      <div class="row"><a class="btn" href="#/spec">Todas las fichas</a><a class="btn primary" href="${format({ view: "build", args: [a.id] })}">${cat.builds[a.id] ? "Ver el montaje" : "Montar"}</a></div></div>
      <div class="split2"><section class="card pad"><h3 class="sec">Asset</h3>${dl([["Id", a.id], ["Petición", a.asked], ["Dónde", a.where], ["Estados", (a.states ?? []).join(", ") || "sin estados"], ["Presupuesto", a.budget_tris != null ? `${fmtN(a.budget_tris)} tri` : "—"], ["Ficha", sp.file]])}${a.notes ? html`<p>${a.notes}</p>` : ""}</section>
      <section class="card pad"><h3 class="sec">Hecho de y dónde se usa</h3>${stale.length ? html`<div class="finding"><span class="pill warn">por rehacer</span> ${stale.map(x => link(cat, x))} ${stale.length > 1 ? "cambiaron" : "cambió"} después del último montaje.</div>` : ""}
        <p><b>Hecho de:</b> ${down.length ? down.map(x => link(cat, x)).reduce((acc, x) => [acc, ", ", x]) : html`<span class="muted">solo piezas del catálogo</span>`}</p>
        <p><b>Se usa en:</b> ${up.length ? up.map(([s, n]) => html`${link(cat, s)}${n > 1 ? ` ×${n}` : ""}`).reduce((acc, x) => [acc, ", ", x]) : html`<span class="muted">ningún otro asset</span>`}</p></section></div>
      <section class="card"><h3 class="sec pad">Piezas <span class="n">${sp.parts.length}</span></h3><table class="tbl"><thead><tr><th>Pieza</th><th>Fuente</th><th>De</th><th>Medida</th><th>Material</th><th>Hallazgos</th></tr></thead><tbody>
        ${sp.parts.map(p => html`<tr class="clickrow" data-part="${p.id}" tabindex="0" aria-label="Pieza ${p.what}"><td><b>${p.what}</b>${p.count > 1 ? ` ×${p.count}` : ""}</td><td>${SOURCE[p.source.kind] ?? p.source.kind}</td><td class="mono">${sourceText(p)}</td><td>${p.size_m ? fmtSize(p.size_m) : "libre"}</td><td class="mono">${p.material ?? "el suyo"}</td><td>${sevBadge(worstOfList(cat.findings.filter(f => f.subject === `part:${a.id}/${p.id}`)))}</td></tr>`)}</tbody></table></section>
      <section class="card pad">${findingsBlock(fs, "Validación de la ficha")}</section>${askForm}</div>`.toString();
  }

  function showPart(pid) {
    const cat = store.get("catalog"), sp = cat.specs.find(s => s.asset.id === id), p = sp?.parts.find(x => x.id === pid);
    if (!p) return;
    selected = pid;
    const from = p.source.from && cat.pieces.find(c => c.key === p.source.from);
    const body = html`<section>${dl([["Id", p.id], ["Medida", p.size_m ? fmtSize(p.size_m) : "—"], ["Cuántas", p.count ?? 1], ["Material", p.material ?? "el suyo"], ["Se mueve", p.moves ?? "no"], ["Fuente", `${SOURCE[p.source.kind] ?? p.source.kind} ${p.source.from ?? p.source.query ?? ""}`], ["Dónde", (p.at ?? []).map(v => "(" + v.map(n => n.toFixed(2)).join(", ") + ")").join(" ") || "sin colocar"], ["Por qué", p.source.why], ["En el catálogo", from ? `${fmtSize(from.size_m)} · ${fmtN(from.tris)} tri · ${(from.licenses ?? []).join(", ")}` : null]])}</section>
      <section class="actions">${from?.preview ? html`<button class="btn" type="button" data-see3d="${from.key}">Ver en 3D</button>` : ""}${canValidate(p, cat) ? html`<button class="btn" type="button" data-validate="${p.id}" title="Deja estos materiales como valor por defecto de ${p.source.from} en el catálogo">Validar materiales</button>` : ""}${p.source.kind === "search" || p.source.from?.includes("/") ? html`<a class="btn" href="${format({ view: "build", args: [id], query: { part: p.id } })}">Buscar en sitios de assets</a>` : ""}${p.source.kind === "asset" && cat.specs.some(s => s.asset.id === p.source.from) ? html`<a class="btn" href="${format({ view: "spec", args: [p.source.from] })}">Entrar en su ficha</a>` : ""}</section>
      ${findingsBlock(cat.findings.filter(f => f.subject === `part:${id}/${p.id}`), "Hallazgos de esta pieza")}`;
    ctx.detail.show({ kind: "Pieza de la ficha", title: p.what, body });
  }

  async function validate(btn) {
    const cat = store.get("catalog"), p = cat.specs.find(s => s.asset.id === id).parts.find(x => x.id === btn.dataset.validate);
    if (!(await ctx.confirm({ title: `¿Validar los materiales de ${p.what}?`, text: `Pasan a ser el valor por defecto de ${p.source.from} en el catálogo y se quitan de la ficha.`, confirm: "Validar" }))) return;
    try {
      const r = await ctx.busy(btn, "Validando los materiales…", () => postJSON(api, "/api/validate", { asset: id, part: p.id }));
      await ctx.reload();
      ctx.toasts.ok("Materiales validados", `Ahora son los de ${r.piece ?? p.source.from} en el catálogo.`);
    } catch (e) { ctx.toasts.error("No se pudo validar", `${e.message} Solo se valida una pieza que viene del catálogo.`); }
  }
  async function ask(form) {
    const input = form.querySelector("input"), text = input.value.trim();
    if (!text) { ctx.toasts.info("Falta qué pedir", "Escribe el asset que quieres, por ejemplo «crear un ascensor»."); input.focus(); return; }
    const btn = form.querySelector("button");
    const job = await ctx.busy(btn, "Enviando la petición…", () => launchJob(ctx, { path: "/api/ask", body: { text }, verb: `desglosar «${text}»` }));
    if (job) input.value = "";
  }

  const onClick = e => {
    const row = e.target.closest("tr[data-part]"); if (row && el.contains(row)) showPart(row.dataset.part);
  };
  const onKey = e => { if (e.key === "Enter" && e.target.matches?.("tr[data-part]")) showPart(e.target.dataset.part); };
  const onSubmit = e => { if (e.target.id === "sAsk") { e.preventDefault(); ask(e.target); } };
  const onDetail = e => {
    const v = e.target.closest("[data-validate]"); if (v) return validate(v);
    const s = e.target.closest("[data-see3d]");
    if (s) { const c = store.get("catalog"), pc = c.pieces.find(x => x.key === s.dataset.see3d); openViewerDialog(doc, { title: pc.name, path: pc.preview, materials: c.materials }); }
  };
  el.addEventListener("click", onClick); el.addEventListener("keydown", onKey); el.addEventListener("submit", onSubmit); detailEl.addEventListener("click", onDetail);
  draw();
  const off = store.subscribe("catalog", () => { draw(); if (selected) showPart(selected); });
  return () => { alive = false; off(); el.removeEventListener("click", onClick); el.removeEventListener("keydown", onKey); el.removeEventListener("submit", onSubmit); detailEl.removeEventListener("click", onDetail); ctx.detail.clear(); el.innerHTML = ""; };
}
