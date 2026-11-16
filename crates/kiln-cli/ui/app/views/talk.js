// The talk panel: one thread with the agent shared by the start wizard and the
// Contexto view. Same endpoints and contract as the old page (/api/chat, the
// job's `reply` and `offer`).
import { html } from "../html.js";
import { postJSON } from "./common.js";
import { pollJob } from "./jobs.js";

const T = { msgs: [], offer: null, busy: false, log: [], loaded: false };
const HINT = { 1: "Pídele nombres, o cuéntale de qué va y que te proponga uno.", 2: "Pregúntale qué cámara o estilo encaja con tu idea, o pídele otras opciones.",
  3: "Cuéntale tu idea a medias y pídele que te ayude a explicarla.", 4: "Pregúntale por qué hace una pregunta, o pídele otras opciones.",
  5: "Pídele cambios en el párrafo o en las afirmaciones; lo que fijaste solo lo propone.", contexto: "Pídele cambios en el contexto; lo que fijaste solo lo propone." };

// The thread as the agent and the project keep it: errors shown here stay here.
export const thread = () => T.msgs.filter(m => !m.local).map(({ who, step, text }) => ({ who, step, text }));
export const resetOffer = () => { T.offer = null; };

export async function loadTalk(api) {
  if (T.loaded) return;
  try { const r = await api.getJSON("/api/chat"); if (r.messages) T.msgs = r.messages; } catch { /* a folder without project has no thread yet */ }
  T.loaded = true;
}
export const markLoaded = () => { T.loaded = true; };

// hooks: { basics(): list, draft(), beforeSend(): Promise<bool>, after(): Promise, apply: { name, basic, text }, addExtra(id, values), rerender() }
export function mountTalk(el, ctx, step, hooks = {}) {
  let alive = true;
  const { api } = ctx;
  function render() {
    if (!alive) return;
    const keep = el.querySelector("#tText")?.value ?? "";
    const o = T.offer ?? {}, basics = hooks.basics?.() ?? [];
    el.innerHTML = html`<div class="kind">Hablar con el agente</div><div class="msgs" role="log" aria-live="polite">${T.msgs.length ? T.msgs.map(m => html`<div class="msg ${m.who === "agent" ? "agent" : "person"}"><small>${m.who === "agent" ? "Agente" : "Tú"} · ${m.step === "contexto" ? "contexto" : "paso " + m.step}</small>${m.text}</div>`) : html`<p class="note">${HINT[step]} Sabe lo que ya has fijado.</p>`}${T.busy ? html`<div class="msg agent busy"><small>El agente trabaja</small>${T.log.slice(-3).join("\n")}</div>` : ""}</div>
      ${o.names?.length && step === "1" ? html`<div class="offer"><small>Nombres</small>${o.names.map(n => html`<button class="btn sm" type="button" data-tname="${n}">${n}</button>`)}</div>` : ""}
      ${step === "2" ? Object.entries(o.basics ?? {}).map(([id, vs]) => { const b = basics.find(x => x.id === id); return b ? html`<div class="offer"><small>${b.label}</small>${vs.map(v => html`<button class="btn sm" type="button" data-tbasic="${id}" data-v="${v}">${v}</button>`)}</div>` : ""; }) : ""}
      ${o.text && (step === "3" || step === "contexto") ? html`<div class="offer"><small>Tu explicación, reescrita</small><button class="btn sm" type="button" id="tUseText">Usar este texto</button></div>` : ""}
      <div class="tin"><textarea class="field" id="tText" placeholder="Pídele algo o discútelo" aria-label="Mensaje para el agente"${T.busy ? " disabled" : ""}>${keep}</textarea><button class="btn primary" type="button" id="tSend"${T.busy ? " disabled" : ""}>Enviar</button></div>`.toString();
    const box = el.querySelector(".msgs"); box.scrollTop = box.scrollHeight;
  }
  async function send() {
    const input = el.querySelector("#tText"), message = input?.value.trim(), early = step === "1" || step === "2";
    if (!message || T.busy) return;
    if (hooks.beforeSend && !(await hooks.beforeSend())) return;
    let r;
    try { r = await postJSON(api, "/api/chat", early ? { step, message, history: thread(), draft: hooks.draft?.() } : { step, message }); }
    catch (e) { T.msgs.push({ who: "agent", step, text: "No se pudo enviar: " + e.message, local: true }); return render(); }
    input.value = "";
    T.msgs.push({ who: "person", step, text: message }); Object.assign(T, { busy: true, log: [], offer: null });
    render();
    let j;
    try { j = await pollJob(api, r.job, { alive: () => alive, onUpdate: x => { T.log = x.log ?? []; render(); } }); }
    catch (e) { j = { status: "failed", log: [e.message] }; }
    T.busy = false;
    if (j.status === "running") return; // the page went away; the job goes on without us
    T.msgs.push(j.status === "done" ? { who: "agent", step, text: j.reply || "(sin respuesta)" } : { who: "agent", step, text: "No pude responder: " + ((j.log ?? []).at(-1) || "error"), local: true });
    T.offer = j.status === "done" ? j.offer ?? null : null;
    for (const [id, vs] of Object.entries(T.offer?.basics ?? {})) hooks.addExtra?.(id, vs);
    if (!early) { await hooks.after?.(); } else if (step === "2") hooks.rerender?.();
    render();
  }
  const onClick = e => {
    if (e.target.closest("#tSend")) return send();
    const n = e.target.closest("[data-tname]"); if (n) return hooks.apply?.name?.(n.dataset.tname);
    const b = e.target.closest("[data-tbasic]"); if (b) return hooks.apply?.basic?.(b.dataset.tbasic, b.dataset.v);
    if (e.target.closest("#tUseText")) hooks.apply?.text?.(T.offer.text);
  };
  const onKey = e => { if (e.target.id === "tText" && e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); } };
  el.addEventListener("click", onClick); el.addEventListener("keydown", onKey);
  render();
  return { render, destroy() { alive = false; el.removeEventListener("click", onClick); el.removeEventListener("keydown", onKey); } };
}
