// Contexto: the project explained by layers: claims (pinned or open),
// questions and every action the old page had, with dialogs and toasts.
// Route: #/context[/layer]
import { html } from "../html.js";
import { format } from "../router.js";
import { postJSON, plural } from "./common.js";
import { pollJob } from "./jobs.js";
import { loadTalk, mountTalk } from "./talk.js";

export const layerName = (data, id) => data.layers.find(l => l.id === id)?.name ?? id;
export const openQuestions = C => C.questions.filter(q => q.status === "open");
export const claimsOf = (C, layer) => C.claims.filter(c => c.layer === layer);
// Layers the project declares, then any layer the claims use that it does not.
export function layersOf(data) {
  const known = new Set(data.layers.map(l => l.id));
  return [...data.layers, ...[...new Set(data.context.claims.map(c => c.layer))].filter(l => !known.has(l)).map(id => ({ id, name: id, holds: "capa que no declara kiln.toml" }))];
}
const sourceText = s => (s.startsWith("answer:") ? "tu respuesta" : s.startsWith("file:") ? s.slice(5) : s === "text" ? "tu explicación" : s);

export function questionCard(q, data) {
  return html`<div class="qcard" data-q="${q.id}"><div class="kind">${layerName(data, q.layer)}</div><b>${q.text}</b>${q.asked_by.startsWith("asset:") ? html`<div class="note">La necesitó el asset ${q.asked_by.slice(6)}</div>` : ""}
    <div class="opts">${q.options.map((o, i) => html`<button class="btn" type="button" data-opt="${i}">${o}</button>`)}</div>
    <div class="free"><input class="field" id="qFree" aria-label="Tu respuesta con tus palabras" placeholder="O con tus palabras" value="${q.answer ?? ""}"><button class="btn primary" type="button" id="qSend">Responder</button></div>
    <div class="skip"><button class="btn sm" type="button" id="qSkip">Saltar</button><button class="btn sm" type="button" id="qNa">No aplica</button></div></div>`;
}

export function claimCard(c, data, { compact = false } = {}) {
  const other = id => data.context.claims.find(x => x.id === id);
  return html`<div class="claim${c.pinned ? " pinned" : ""}" data-claim="${c.id}"><div class="ct">${c.pinned ? html`<span class="pill ok">fijada</span> ` : ""}${c.conflicts ? html`<span class="pill error">choca con otra</span> ` : ""}${c.text}</div>
    <div class="muted small">${sourceText(c.source)}${compact ? "" : html` · ${layerName(data, c.layer)}`}</div>
    ${c.conflicts && other(c.conflicts) ? html`<div class="finding"><span class="muted">Choca con</span> ${other(c.conflicts).text}</div>` : ""}
    ${c.proposed ? html`<div class="proposal"><b>El agente propone:</b> ${c.proposed}<div class="row"><button class="btn sm" type="button" data-cact="accept" data-id="${c.id}">Aceptar</button><button class="btn sm" type="button" data-cact="reject" data-id="${c.id}">Descartar</button></div></div>` : ""}
    <div class="row"><button class="btn sm" type="button" data-cact="edit" data-id="${c.id}">Editar y fijar</button><button class="btn sm" type="button" data-cact="${c.pinned ? "unpin" : "pin"}" data-id="${c.id}">${c.pinned ? "Soltar" : "Fijar"}</button><button class="btn sm" type="button" data-cact="delete" data-id="${c.id}">Borrar</button></div></div>`;
}

// One action on the context: post, reload, toast. A failure says what to do.
export async function contextAction(ctx, control, { path, body, ok, verb }) {
  try {
    await (control ? ctx.busy(control, "Guardando…", () => postJSON(ctx.api, `/api/context/${path}`, body)) : postJSON(ctx.api, `/api/context/${path}`, body));
    await ctx.reload();
    if (ok) ctx.toasts.ok(ok);
    return true;
  } catch (e) { ctx.toasts.error(`No se pudo ${verb}`, `${e.message} Comprueba el texto y vuelve a intentarlo.`); return false; }
}

// A job of the agent (sort, refine): started, followed, and its result told.
export async function contextJob(ctx, path, body, { alive = () => true, onTick } = {}) {
  let r;
  try { r = await postJSON(ctx.api, `/api/context/${path}`, body ?? {}); } catch (e) { ctx.toasts.error("No se pudo lanzar el trabajo", `${e.message} Vuelve a intentarlo.`); return null; }
  await ctx.refreshJobs();
  ctx.toasts.info("El agente trabaja", "Puedes seguirlo en Trabajos.", { label: "Ver el trabajo", run: () => ctx.navigate({ view: "jobs", args: [String(r.job)] }) });
  let j;
  try { j = await pollJob(ctx.api, r.job, { alive, onUpdate: onTick }); } catch (e) { ctx.toasts.error("No se pudo seguir el trabajo", e.message); return null; }
  if (!alive() && j.status === "running") return j;
  await ctx.reload().catch(() => {});
  if (j.status === "done") ctx.toasts.ok("El agente terminó", j.asked);
  else ctx.toasts.error("El trabajo falló", `${(j.log ?? []).at(-1) ?? "sin detalle"}. Revisa el registro en Trabajos y vuelve a lanzarlo.`, { label: "Ver el trabajo", run: () => ctx.navigate({ view: "jobs", args: [String(r.job)] }) });
  return j;
}

export function mount(el, ctx) {
  const { store } = ctx;
  let layer = ctx.route.args?.[0] ?? "", dirty = false, talk = null, alive = true, working = false;
  store.set("panel", false); // the view has no detail: the room goes to the content
  const data = () => store.get("context");
  const sub = () => ctx.askText;

  function draw() {
    const d = data();
    if (!d) { el.innerHTML = ""; return; }
    const C = d.context, layers = layersOf(d), qs = openQuestions(C);
    if (layer && !layers.some(l => l.id === layer)) layer = "";
    const shown = layer ? layers.filter(l => l.id === layer) : layers;
    const keep = dirty ? el.querySelector("#xText")?.value : null;
    const q = layer ? qs.filter(x => x.layer === layer) : qs;
    el.innerHTML = html`<div class="page" data-view="context" data-ready="1">
      <div class="pagehead"><div><h1>Contexto</h1><div class="sub">${plural(C.claims.length, "afirmación", "afirmaciones")} en ${plural(new Set(C.claims.map(c => c.layer)).size, "capa", "capas")} · ${plural(qs.length, "pregunta abierta", "preguntas abiertas")}</div></div></div>
      <div class="ctxgrid"><nav class="sidelist card" id="xLayers" aria-label="Capas"><h2>Capas</h2><a href="${format({ view: "context" })}" data-layer=""${!layer ? html` aria-current="true"` : ""}><span class="dot none" aria-hidden="true"></span><span>Todas las capas</span><span class="cnt">${C.claims.length}</span></a><hr>
        ${layers.map(l => html`<a href="${format({ view: "context", args: [l.id] })}" data-layer="${l.id}" title="${l.holds}"${layer === l.id ? html` aria-current="true"` : ""}><span class="dot ${claimsOf(C, l.id).length ? "ok" : "warn"}" aria-hidden="true"></span><span>${l.name}</span><span class="cnt">${claimsOf(C, l.id).length}</span></a>`)}</nav>
        <div class="ctxmain">
          <section class="card pad"><h3 class="sec">Explica el juego</h3><p class="muted small">De qué va, época, estilo, qué ve y hace el jugador, reglas de los assets. «Ordenar» lo parte en capas; «Afinar» te hace preguntas. Se guarda en ${d.file}.</p>
            <textarea class="field" id="xText" aria-label="Explicación del juego" placeholder="Un juego de...">${keep ?? C.text}</textarea>
            <div class="row"><button class="btn" type="button" data-x="save">Guardar</button><button class="btn primary" type="button" data-x="sort">Guardar y ordenar</button><button class="btn" type="button" data-x="refine"${C.claims.length ? "" : " disabled"}>Afinar con preguntas</button></div></section>
          ${C.intent?.text || C.intent?.proposed ? html`<section class="card pad"><h3 class="sec">Qué juego es ${C.intent.pinned ? html`<span class="pill ok">fijado</span>` : ""}</h3><p>${C.intent.text}</p>
            ${C.intent.proposed ? html`<div class="proposal"><b>El agente propone:</b> ${C.intent.proposed}<div class="row"><button class="btn sm" type="button" data-x="iacc">Aceptar</button><button class="btn sm" type="button" data-x="irej">Descartar</button></div></div>` : ""}
            <div class="row"><button class="btn sm" type="button" data-x="iedit">Editar y fijar el párrafo</button></div></section>` : ""}
          <section class="card pad" id="xQs"><h3 class="sec">Preguntas ${q.length ? html`<span class="n">${q.length}</span>` : ""}</h3>
            ${q.length ? html`<p class="muted small">Pregunta ${C.questions.length - qs.length + 1} de ${C.questions.length}${layer ? "" : ""}</p>${questionCard(q[0], d)}` : html`<p class="muted">${C.claims.length ? "No hay preguntas abiertas. «Afinar» pide otra ronda." : "Cuando haya contexto, el agente podrá hacerte preguntas."}</p>`}
            ${layer ? html`<div class="row"><button class="btn sm" type="button" data-x="layerask">Pedir preguntas sobre esta capa</button></div>` : ""}</section>
          ${shown.map(l => html`<section class="card pad" data-layer-sec="${l.id}"><h3 class="sec">${l.name} <span class="n">${claimsOf(C, l.id).length}</span></h3><p class="muted small">${l.holds}</p>
            ${claimsOf(C, l.id).length ? claimsOf(C, l.id).map(c => claimCard(c, d, { compact: true })) : html`<p class="muted">Vacía.</p>`}
            <div class="row"><button class="btn sm" type="button" data-x="add" data-layer="${l.id}">Añadir una afirmación</button></div></section>`)}
        </div><aside class="talk" id="xTalk" aria-label="Hablar con el agente"></aside></div></div>`.toString();
    talk?.destroy();
    talk = mountTalk(el.querySelector("#xTalk"), ctx, "contexto", { after: () => ctx.reload(), apply: { text: t => { const b = el.querySelector("#xText"); if (b) { b.value = t; dirty = true; } } } });
  }

  const text = () => el.querySelector("#xText").value;
  const act = (control, path, body, ok, verb) => contextAction(ctx, control, { path, body, ok, verb });
  const runJob = async (control, path, body) => {
    if (working) return; working = true;
    document.querySelectorAll("[data-x=sort],[data-x=refine],[data-x=layerask]").forEach(b => { b.disabled = true; });
    try { await contextJob(ctx, path, body, { alive: () => alive }); } finally { working = false; }
  };
  const claimAct = async (b) => {
    const id = b.dataset.id, a = b.dataset.cact, c = data().context.claims.find(x => x.id === id);
    if (a === "delete") { if (!(await ctx.confirm({ title: "¿Borrar esta afirmación?", text: c?.text ?? "", confirm: "Borrar" }))) return; return act(b, "claim", { id, action: "delete" }, "Afirmación borrada", "borrar la afirmación"); }
    if (a === "edit") { const t = await sub()({ title: "Editar la afirmación", label: "Texto", initial: c?.text ?? "", confirm: "Guardar y fijar", validate: v => (v ? "" : "Escribe el texto de la afirmación.") }); if (t == null) return; return act(b, "claim", { id, action: "edit", text: t }, "Afirmación guardada y fijada", "guardar la afirmación"); }
    return act(b, "claim", { id, action: a, text: "" }, { pin: "Afirmación fijada", unpin: "Afirmación suelta", accept: "Propuesta aceptada", reject: "Propuesta descartada" }[a], "cambiar la afirmación");
  };
  const answer = (b, body) => act(b, "answer", body, "Respuesta guardada", "guardar la respuesta");
  const onClick = async e => {
    const x = e.target.closest("[data-x]"), c = e.target.closest("[data-cact]"), o = e.target.closest("[data-opt]"), qd = e.target.closest(".qcard");
    if (c) return claimAct(c);
    if (o && qd) { const q = data().context.questions.find(v => v.id === qd.dataset.q); return answer(o, { id: q.id, answer: q.options[+o.dataset.opt], status: "answered" }); }
    if (qd && e.target.closest("#qSend")) return answer(e.target.closest("#qSend"), { id: qd.dataset.q, answer: qd.querySelector("#qFree").value, status: "answered" });
    if (qd && e.target.closest("#qSkip")) return answer(e.target.closest("#qSkip"), { id: qd.dataset.q, status: "skipped" });
    if (qd && e.target.closest("#qNa")) return answer(e.target.closest("#qNa"), { id: qd.dataset.q, status: "not_applicable" });
    if (!x) return;
    const k = x.dataset.x;
    if (k === "save") { if (await act(x, "text", { text: text() }, "Contexto guardado", "guardar el contexto")) dirty = false; }
    else if (k === "sort") { if (await act(x, "text", { text: text() }, null, "guardar el contexto")) { dirty = false; runJob(x, "sort"); } }
    else if (k === "refine") runJob(x, "refine");
    else if (k === "layerask") runJob(x, "refine", { layer });
    else if (k === "add") { const t = await sub()({ title: "Añadir una afirmación", text: `A la capa ${layerName(data(), x.dataset.layer)}.`, label: "Afirmación", confirm: "Añadir", validate: v => (v ? "" : "Escribe la afirmación.") }); if (t != null) act(x, "claim", { action: "add", layer: x.dataset.layer, text: t }, "Afirmación añadida", "añadir la afirmación"); }
    else if (k === "iedit") { const t = await sub()({ title: "Fijar el párrafo", label: "Qué juego es", initial: data().context.intent.text, confirm: "Guardar y fijar", validate: v => (v ? "" : "Escribe el párrafo.") }); if (t != null) act(x, "intent", { action: "edit", text: t }, "Párrafo fijado", "fijar el párrafo"); }
    else if (k === "iacc") act(x, "intent", { action: "accept" }, "Propuesta aceptada", "aceptar la propuesta");
    else if (k === "irej") act(x, "intent", { action: "reject" }, "Propuesta descartada", "descartar la propuesta");
  };
  const onInput = e => { if (e.target.id === "xText") dirty = true; };
  const onKey = e => { if (e.target.id === "qFree" && e.key === "Enter") el.querySelector("#qSend")?.click(); };
  const onLink = e => {
    const a = e.target.closest("#xLayers a[data-layer]");
    if (!a || e.button !== 0 || e.ctrlKey || e.metaKey) return;
    e.preventDefault(); layer = a.dataset.layer; history.replaceState(null, "", a.getAttribute("href")); draw();
  };
  el.addEventListener("click", onClick); el.addEventListener("click", onLink); el.addEventListener("input", onInput); el.addEventListener("keydown", onKey);
  loadTalk(ctx.api).then(() => alive && draw());
  draw();
  const off = store.subscribe("context", draw);
  return () => { alive = false; off(); talk?.destroy(); el.removeEventListener("click", onClick); el.removeEventListener("click", onLink); el.removeEventListener("input", onInput); el.removeEventListener("keydown", onKey); el.innerHTML = ""; };
}
