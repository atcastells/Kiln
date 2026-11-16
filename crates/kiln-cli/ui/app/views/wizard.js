// The start wizard (`#/new`, opened by itself when the folder has no kiln.toml
// or its start is not settled): five steps and the talk panel, with the same
// endpoints and contract as the old page.
import { html } from "../html.js";
import { postJSON } from "./common.js";
import { pollJob } from "./jobs.js";
import { questionCard, claimCard } from "./context.js";
import { loadTalk, mountTalk, thread, markLoaded, resetOffer } from "./talk.js";

export const STEPS = ["Proyecto", "Lo básico", "Tu juego", "Preguntas", "Asentar"];
// The step to show: what the server says, but never back from one the person reached.
export const wizardStep = (info, prev) => (info.step === 1 ? (prev === 2 ? 2 : 1) : Math.max(info.step, prev));
export const stepStates = step => STEPS.map((title, i) => ({ n: i + 1, title, state: i + 1 < step ? "done" : i + 1 === step ? "on" : "todo" }));
export const optionsOf = (b, extra = {}) => [...b.options, ...(extra[b.id] ?? []).filter(o => !b.options.includes(o))];

const wz = { step: 1, name: "", basics: {}, extra: {}, info: null, data: null };

export function mount(el, ctx) {
  const { api, store, toasts } = ctx;
  let talk = null, alive = true, working = false;
  const q = sel => el.querySelector(sel);

  async function load() {
    api.forget("/api/new");
    const n = await api.getJSON("/api/new");
    if (!n.step) return false;
    wz.info = { ...wz.info, ...n };
    wz.step = wizardStep(n, wz.step);
    if (wz.step > 2) {
      api.forget("/api/context");
      wz.data = await api.getJSON("/api/context");
      if (!wz.name) { api.forget("/api/catalog"); wz.name = (await api.getJSON("/api/catalog")).project ?? ""; }
      await loadTalk(api);
    }
    return true;
  }
  async function refresh() {
    try { if (!(await load())) return finished(); } catch (e) { toasts.error("No se pudo leer el estado del asistente", `${e.message} Recarga la página para volver a intentarlo.`); }
    if (alive) draw();
  }
  function finished() {
    el.innerHTML = html`<div class="page" data-view="wizard" data-ready="1"><div class="empty"><h3>Este proyecto ya está asentado</h3><p>El asistente de inicio solo se abre en una carpeta nueva o con el inicio sin asentar.</p><a class="btn primary" href="#/home">Ir al inicio</a></div></div>`.toString();
  }
  const fail = (e, verb) => { toasts.error(`No se pudo ${verb}`, `${e.message} Comprueba los datos y vuelve a intentarlo.`); return false; };
  const post = async (path, body, verb) => { try { return await postJSON(api, path, body); } catch (e) { fail(e, verb); return null; } };
  async function ctxPost(what, body, verb) {
    if (!(await post("/api/context/" + what, body, verb))) return false;
    api.forget("/api/context"); wz.data = await api.getJSON("/api/context"); return true;
  }
  // A job of the agent with its log on the card; true when it finished well.
  async function runJob(what) {
    const r = await post("/api/context/" + what, {}, "lanzar al agente"); if (!r) return false;
    let j;
    try {
      j = await pollJob(api, r.job, { alive: () => alive, onUpdate: x => {
        const box = q("#wzLog"); if (box) box.innerHTML = html`<div class="kind" role="status">${x.status === "running" ? "El agente trabaja" : x.status === "done" ? "Hecho" : "Falló"} · ${x.asked}</div>${(x.log ?? []).map(l => html`<div class="log">${l}</div>`)}`.toString();
      } });
    } catch (e) { return fail(e, "seguir al agente"); }
    api.forget("/api/context"); wz.data = await api.getJSON("/api/context");
    if (j.status !== "done") toasts.error("El agente no terminó", `${(j.log ?? []).at(-1) ?? "sin detalle"}. Puedes volver a pulsar el botón.`);
    return j.status === "done";
  }

  function frame(body, nav) {
    el.innerHTML = html`<div class="page wizard" data-view="wizard" data-ready="1" data-step="${wz.step}"><div class="wzgrid"><div class="wz"><ol class="steps" aria-label="Paso ${wz.step} de 5">${stepStates(wz.step).map(s => html`<li class="${s.state === "todo" ? "" : s.state}"${s.state === "on" ? html` aria-current="step"` : ""}>${s.n} · ${s.title}</li>`)}</ol>
      <div class="card pad">${body}<div class="nav">${nav}</div></div></div><aside class="talk" id="wzTalk" aria-label="Hablar con el agente"></aside></div></div>`.toString();
    mountTalkPanel();
  }
  function mountTalkPanel() {
    talk?.destroy();
    const step = String(wz.step);
    talk = mountTalk(q("#wzTalk"), ctx, step, {
      basics: () => wz.info?.basics ?? [],
      draft: () => ({ name: wz.name, basics: wz.basics }),
      beforeSend: async () => {
        if (step === "1" && q("#wzName")) wz.name = q("#wzName").value.trim();
        if (step === "3" && q("#wzText") && q("#wzText").value.trim() !== (wz.data?.context?.text ?? "").trim()) return ctxPost("text", { text: q("#wzText").value }, "guardar tu explicación");
        return true;
      },
      after: async () => { api.forget("/api/context"); wz.data = await api.getJSON("/api/context"); wz.info = { ...wz.info, ...(await (api.forget("/api/new"), api.getJSON("/api/new"))), step: wz.step }; if (alive) draw(); },
      apply: { name: n => { wz.name = n; if (q("#wzName")) q("#wzName").value = n; }, basic: (id, v) => { wz.basics[id] = v; draw(); }, text: t => { if (q("#wzText")) q("#wzText").value = t; } },
      addExtra: (id, vs) => { wz.extra[id] = [...new Set([...(wz.extra[id] ?? []), ...vs])]; },
      rerender: () => draw(),
    });
  }

  function draw() {
    const s = wz.step, d = wz.data, C = d?.context;
    store.set("wizardName", wz.name);
    const top = q("#kiln-project") ?? document.getElementById("kiln-project"); if (top) top.textContent = wz.name || "juego nuevo";
    if (s === 1) {
      frame(html`<h2>Un juego nuevo</h2><p class="muted">En <span class="path">${wz.info.folder}</span>. Kiln escribe aquí su <span class="path">kiln.toml</span> y el contexto del juego; no toca nada más.</p>
        <label class="wzfield">Nombre del juego<input class="field" id="wzName" value="${wz.name}" placeholder="Puerto Niebla" autocomplete="off"></label>
        <div class="facts"><div class="fact"><span>Tipo</span><b>Juego</b><small>De momento, el único tipo</small></div><div class="fact"><span>Motor</span><b>Godot 4</b></div></div>`, html`<button class="btn primary" type="button" id="wzNext">Seguir</button>`);
      q("#wzName").focus();
    } else if (s === 2) {
      frame(html`<h2>Lo básico</h2><p class="muted">Cinco cosas que cualquier asset necesita saber. Elige una o escribe la tuya; puedes dejar alguna en blanco.</p>
        ${wz.info.basics.map(b => html`<div class="qcard"><b>${b.text}</b><div class="opts">${optionsOf(b, wz.extra).map((o, i) => html`<button class="btn${wz.basics[b.id] === o ? " on" : ""}" type="button" data-b="${b.id}" data-i="${i}" aria-pressed="${String(wz.basics[b.id] === o)}">${o}</button>`)}</div>
          <div class="free"><input class="field" data-free="${b.id}" aria-label="${b.text}: con tus palabras" placeholder="O con tus palabras" value="${optionsOf(b, wz.extra).includes(wz.basics[b.id]) ? "" : wz.basics[b.id] ?? ""}"></div></div>`)}`,
        html`<button class="btn" type="button" id="wzBack">Atrás</button><button class="btn primary" type="button" id="wzMake">Crear el proyecto</button>`);
    } else if (s === 3) {
      frame(html`<h2>Tu juego, con tus palabras</h2><p class="muted">De qué va, dónde y cuándo pasa, qué hace el jugador y cómo se ve. Si se parece a un juego que existe, dilo. El agente lo parte en afirmaciones por capa, escribe un párrafo de qué juego es y te hace preguntas.</p>
        <textarea class="field" id="wzText" aria-label="Tu juego con tus palabras" placeholder="Un juego de...">${C.text}</textarea><div id="wzLog"></div>`, html`<button class="btn primary" type="button" id="wzSort">Ordenar y preguntar</button>`);
    } else if (s === 4) {
      const qs = C.questions.filter(x => x.status === "open");
      if (!qs.length) { wz.step = 5; return draw(); }
      frame(html`<h2>Unas preguntas</h2><p class="muted">Quedan ${qs.length}. Cada respuesta queda fijada en su capa: el agente no la cambia.</p>${questionCard(qs[0], d)}<div id="wzLog"></div>`,
        html`<button class="btn" type="button" id="wzMore">Otra ronda</button><button class="btn primary" type="button" id="wzSkipAll">Seguir sin responder</button>`);
    } else {
      const I = C.intent ?? {}, missing = wz.info.missing ?? [];
      frame(html`<h2>Asentar el inicio</h2><p class="muted">Esto es lo que todo asset leerá primero. Fija el párrafo y al menos una afirmación por capa.</p>
        <div class="kind">Qué juego es ${I.pinned ? html`<span class="pill ok">fijado</span>` : ""}</div>
        <textarea class="field" id="wzIntent" aria-label="Qué juego es" style="min-height:110px">${I.text}</textarea>
        <div class="row"><button class="btn" type="button" id="wzPinI">Guardar y fijar el párrafo</button></div>
        ${I.proposed ? html`<div class="proposal"><b>El agente propone:</b> ${I.proposed}<div class="row"><button class="btn sm" type="button" id="wzAcc">Aceptar</button><button class="btn sm" type="button" id="wzRej">Descartar</button></div></div>` : ""}
        <div class="kind">Por capa</div>
        ${d.layers.map(l => { const cs = C.claims.filter(c => c.layer === l.id); return html`<div class="lrow"><b>${l.name} ${cs.some(c => c.pinned) ? html`<span class="pill ok">fijada</span>` : html`<span class="pill warn">nada fijado</span>`}</b>${cs.map(c => claimCard(c, d, { compact: true }))}<div class="free"><input class="field" data-add="${l.id}" aria-label="Añadir una afirmación a ${l.name}" placeholder="Añadir una afirmación a ${l.name}"></div></div>`; })}
        ${missing.length ? html`<div class="finding"><b>Falta:</b> ${missing.join(" · ")}</div>` : html`<p class="muted">Todo listo.</p>`}`,
        html`<button class="btn" type="button" id="wzQs">Más preguntas</button><button class="btn primary" type="button" id="wzSettle"${missing.length ? " disabled" : ""}>Asentar y abrir el proyecto</button>`);
    }
  }

  const busyRun = async (btn, reason, fn) => { if (working) return; working = true; try { await ctx.busy(btn, reason, fn); } finally { working = false; } };
  const onClick = async e => {
    const t = e.target.closest("button"); if (!t || !el.contains(t)) return;
    const id = t.id, s = wz.step;
    if (id === "wzNext") { wz.name = q("#wzName").value.trim(); if (wz.name) { wz.step = 2; draw(); } else { toasts.info("Falta el nombre", "Escribe cómo se llama el juego para seguir."); q("#wzName").focus(); } }
    else if (id === "wzBack") { wz.step = 1; draw(); }
    else if (t.dataset.b) { const b = wz.info.basics.find(x => x.id === t.dataset.b); wz.basics[b.id] = optionsOf(b, wz.extra)[+t.dataset.i]; draw(); }
    else if (id === "wzMake") await busyRun(t, "Creando el proyecto…", async () => {
      const r = await post("/api/new", { name: wz.name, basics: wz.basics, chat: thread() }, "crear el proyecto"); if (!r) return;
      markLoaded(); wz.step = 3; await refresh();
    });
    else if (id === "wzSort") await busyRun(t, "El agente ordena y pregunta…", async () => {
      const text = q("#wzText").value.trim(); if (!text) { q("#wzText").focus(); return; }
      if (!(await ctxPost("text", { text }, "guardar tu explicación"))) return;
      q("#wzText").disabled = true;
      if (!(await runJob("sort")) || !(await runJob("refine"))) { if (alive && q("#wzText")) q("#wzText").disabled = false; return; }
      await refresh();
    });
    else if (id === "wzMore") await busyRun(t, "El agente prepara otra ronda…", async () => { if (await runJob("refine")) draw(); });
    else if (id === "wzSkipAll") { wz.step = 5; draw(); }
    else if (id === "wzQs") await busyRun(t, "El agente prepara preguntas…", async () => { if (await runJob("refine")) { wz.step = 4; await refresh(); } });
    else if (id === "wzPinI") { if (await ctxPost("intent", { action: "edit", text: q("#wzIntent").value }, "fijar el párrafo")) await refresh(); }
    else if (id === "wzAcc" || id === "wzRej") { if (await ctxPost("intent", { action: id === "wzAcc" ? "accept" : "reject" }, "responder a la propuesta")) await refresh(); }
    else if (id === "wzSettle") await busyRun(t, "Asentando el inicio…", async () => {
      if (!(await post("/api/settle", {}, "asentar el inicio"))) return;
      toasts.ok("Inicio asentado", "El proyecto está listo.");
      resetOffer(); await ctx.leaveWizard();
    });
    else if (t.dataset.cact) {
      const cid = t.dataset.id, a = t.dataset.cact;
      if (a === "delete" && !(await ctx.confirm({ title: "¿Borrar esta afirmación?", confirm: "Borrar" }))) return;
      if (a === "edit") { const c = wz.data.context.claims.find(x => x.id === cid); const v = await ctx.askText({ title: "Editar la afirmación", label: "Texto", initial: c.text, confirm: "Guardar y fijar", validate: x => (x ? "" : "Escribe el texto.") }); if (v == null) return; if (await ctxPost("claim", { id: cid, action: "edit", text: v }, "guardar la afirmación")) await refresh(); return; }
      if (await ctxPost("claim", { id: cid, action: a }, "cambiar la afirmación")) await refresh();
    } else if (t.dataset.opt !== undefined && s === 4) {
      const qd = t.closest(".qcard"), qq = wz.data.context.questions.find(x => x.id === qd.dataset.q);
      if (await ctxPost("answer", { id: qq.id, answer: qq.options[+t.dataset.opt], status: "answered" }, "guardar la respuesta")) await refresh();
    } else if (s === 4 && ["qSend", "qSkip", "qNa"].includes(id)) {
      const qd = t.closest(".qcard"), body = { id: qd.dataset.q, ...(id === "qSend" ? { answer: qd.querySelector("#qFree").value, status: "answered" } : { status: id === "qSkip" ? "skipped" : "not_applicable" }) };
      if (await ctxPost("answer", body, "guardar la respuesta")) await refresh();
    }
  };
  const onKey = async e => {
    if (e.key !== "Enter") return;
    if (e.target.id === "wzName") q("#wzNext").click();
    else if (e.target.id === "qFree") q("#qSend").click();
    else if (e.target.dataset?.add && e.target.value.trim()) { if (await ctxPost("claim", { action: "add", layer: e.target.dataset.add, text: e.target.value }, "añadir la afirmación")) await refresh(); }
  };
  const onInput = e => { if (e.target.dataset?.free && e.target.value.trim()) wz.basics[e.target.dataset.free] = e.target.value.trim(); };
  el.addEventListener("click", onClick); el.addEventListener("keydown", onKey); el.addEventListener("input", onInput);
  refresh();
  return () => { alive = false; talk?.destroy(); el.removeEventListener("click", onClick); el.removeEventListener("keydown", onKey); el.removeEventListener("input", onInput); el.innerHTML = ""; };
}
