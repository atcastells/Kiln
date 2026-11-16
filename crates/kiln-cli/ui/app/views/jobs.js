// Trabajos: the list of jobs and the follow-up of one (`#/jobs[/id]`). The
// route holds the id, so a reload picks the job up again. Polling backs off
// from 1 s to 5 s; the log only follows the end while the person is there.
import { html } from "../html.js";
import { format } from "../router.js";
import { postJSON } from "./common.js";

export const POLL_FIRST = 1000, POLL_MAX = 5000;
export const nextDelay = d => Math.min(POLL_MAX, Math.round(d * 1.5));
export const schedule = n => { const out = []; for (let d = POLL_FIRST, i = 0; i < n; i++, d = nextDelay(d)) out.push(d); return out; };
// True while the log is scrolled to its end (a few pixels of slack).
export const atBottom = ({ scrollTop, scrollHeight, clientHeight }, slack = 24) => scrollHeight - scrollTop - clientHeight <= slack;

export const STATE = { running: "en marcha", done: "hecho", failed: "falló" };
const dirOf = p => p.replace(/[^/\\]+$/, "");

// Where to go after a job finished, by what it worked on (`job.spec`).
export function nextStep(job, cat, context) {
  const ok = job.status === "done", spec = job.spec ?? "";
  const builds = cat?.builds ?? {};
  let target = null;
  if (context?.file && spec === context.file) target = { route: { view: "context" }, label: "Ver el contexto" };
  else if (spec.endsWith("review/review.json")) {
    const id = Object.keys(builds).find(k => spec.startsWith(dirOf(builds[k].glb)) || spec.includes(`/${k}/review/`));
    if (id) target = { route: { view: "build", args: [id] }, label: "Ver el montaje y la revisión" };
  } else if (spec.endsWith(".glb")) {
    const id = Object.keys(builds).find(k => builds[k].glb === spec || Object.values(builds[k].variants ?? {}).includes(spec));
    if (id) target = { route: { view: "build", args: [id] }, label: "Ver el montaje" };
  } else if (spec) {
    const sp = (cat?.specs ?? []).find(x => x.file === spec);
    if (sp) target = { route: { view: "spec", args: [sp.asset.id] }, label: "Ver la ficha" };
  }
  const text = ok ? "El trabajo terminó." : "El trabajo falló: el final del registro dice por qué. Corrige eso y vuelve a lanzarlo desde su vista.";
  return { ok, text, target };
}

// Follows a job until it stops running. `onUpdate(job)` runs on each read.
export async function pollJob(api, id, { onUpdate = () => {}, sleep = ms => new Promise(r => setTimeout(r, ms)), alive = () => true } = {}) {
  let delay = POLL_FIRST;
  for (;;) {
    api.forget(`/api/jobs/${id}`);
    const job = await api.getJSON(`/api/jobs/${id}`);
    if (!alive()) return job;
    onUpdate(job);
    if (job.status !== "running") return job;
    await sleep(delay);
    if (!alive()) return job;
    delay = nextDelay(delay);
  }
}

// Starts a job (`POST path`), says so with a way to follow it, and tells how it ended.
export async function launchJob(ctx, { path, body, verb, done }) {
  let r;
  try { r = await postJSON(ctx.api, path, body); }
  catch (e) { ctx.toasts.error(`No se pudo ${verb}`, `${e.message} Comprueba que la herramienta necesaria está instalada y vuelve a intentarlo.`); return null; }
  await ctx.refreshJobs();
  const go = { label: "Ver el trabajo", run: () => ctx.navigate({ view: "jobs", args: [String(r.job)] }) };
  ctx.toasts.info(`En marcha: ${verb}`, "Puedes seguirlo en Trabajos.", go);
  pollJob(ctx.api, r.job).then(async j => {
    await ctx.reload().catch(() => {});
    if (j.status === "done") ctx.toasts.ok(`Terminó: ${verb}`, j.asked, go);
    else ctx.toasts.error(`Falló: ${verb}`, `${(j.log ?? []).at(-1) ?? "sin detalle"}. El registro dice más; corrige y vuelve a lanzarlo.`, go);
    done?.(j);
  }).catch(e => ctx.toasts.error("No se pudo seguir el trabajo", e.message));
  return r.job;
}

const when = s => (s ? new Date(s * 1000).toLocaleString("es", { dateStyle: "short", timeStyle: "short" }) : "");

export function mount(el, ctx) {
  const { store, api } = ctx;
  const id = ctx.route.args?.[0] ?? null;
  let alive = true, job = null, missing = false, stick = true, shown = -1;
  store.set("panel", false);
  const link = j => format({ view: "jobs", args: [String(j.id)] });

  function paintList() {
    const list = [...(store.get("jobs") ?? [])].reverse();
    el.querySelector("#jList").innerHTML = list.length ? list.map(j => html`<a href="${link(j)}" data-job="${j.id}" title="${when(j.started)}"${String(j.id) === id ? html` aria-current="true"` : ""}><span class="dot ${j.state === "running" ? "busy" : j.state === "failed" ? "error" : "ok"}" aria-hidden="true"></span><span>${j.label}<span class="sr"> (${STATE[j.state] ?? j.state})</span></span></a>`).join("")
      : html`<p class="muted small" style="padding:8px 12px">Todavía no hay trabajos.</p>`.toString();
  }
  function paintDetail() {
    const box = el.querySelector("#jDetail");
    if (!id) { box.innerHTML = html`<div class="empty"><h2>${(store.get("jobs") ?? []).length ? "Elige un trabajo" : "No hay trabajos"}</h2><p>${(store.get("jobs") ?? []).length ? "Aquí ves su registro mientras corre y lo que hacer cuando termina." : "Cuando pidas un asset, montes o revises, el trabajo aparece aquí y puedes seguirlo, incluso si recargas la página."}</p></div>`.toString(); return; }
    if (missing) { box.innerHTML = html`<div class="empty"><h2>Ese trabajo ya no existe</h2><p>El servidor se reinició o el trabajo es muy antiguo. Mira la lista de la izquierda.</p></div>`.toString(); return; }
    if (!job) { box.innerHTML = html`<p class="muted">Cargando el trabajo…</p>`.toString(); return; }
    const log = job.log ?? [];
    if (!box.querySelector("#jLog")) {
      box.innerHTML = html`<div class="jhead"><h2 id="jTitle"></h2><span class="sev" id="jState"></span></div>
        <div class="logwrap"><div class="log" id="jLog" role="log" aria-live="polite" tabindex="0" aria-label="Registro del trabajo"></div><button class="btn sm tail" type="button" id="jTail" hidden>Ir al final</button></div><div id="jNext"></div>`.toString();
      box.querySelector("#jLog").addEventListener("scroll", () => { stick = atBottom(box.querySelector("#jLog")); if (stick) box.querySelector("#jTail").hidden = true; });
      box.querySelector("#jTail").addEventListener("click", () => { const l = box.querySelector("#jLog"); l.scrollTop = l.scrollHeight; stick = true; box.querySelector("#jTail").hidden = true; });
    }
    box.querySelector("#jTitle").textContent = `«${job.asked}»`;
    const st = box.querySelector("#jState");
    st.className = `sev ${job.status === "failed" ? "error" : job.status === "running" ? "info" : "ok"}`;
    st.textContent = STATE[job.status] ?? job.status;
    const l = box.querySelector("#jLog");
    if (log.length !== shown) {
      l.innerHTML = log.map(x => html`<div>${x}</div>`).join("") || html`<div class="muted">Sin salida todavía…</div>`.toString();
      if (stick) l.scrollTop = l.scrollHeight; else box.querySelector("#jTail").hidden = false;
      shown = log.length;
    }
    const nx = box.querySelector("#jNext");
    if (job.status === "running") nx.innerHTML = "";
    else {
      const s = nextStep(job, store.get("catalog"), store.get("context"));
      nx.innerHTML = html`<div class="callout ${s.ok ? "" : "bad"}" role="status"><b>${s.ok ? "Hecho" : "Falló"}</b><p>${s.text}</p>${job.reply ? html`<p>${job.reply}</p>` : ""}${s.target ? html`<a class="btn primary" href="${format(s.target.route)}">${s.target.label}</a>` : ""}</div>`.toString();
    }
  }
  el.innerHTML = html`<div class="page fill" data-view="jobs" data-ready="1"><div class="pagehead"><div><h1>Trabajos</h1><div class="sub">Lo que Kiln hace por ti: sigue su registro y vuelve cuando quieras</div></div></div>
    <div class="split"><nav class="sidelist card" id="jList" aria-label="Trabajos"></nav><div class="pane card jpane" id="jDetail" style="padding:var(--s4)"></div></div></div>`.toString();
  paintList(); paintDetail();
  const off = store.subscribe("jobs", paintList);
  if (id) {
    (async () => {
      try {
        const done = await pollJob(api, id, { alive: () => alive, onUpdate: j => { job = j; paintDetail(); } });
        if (alive && done.status !== "running") { await ctx.reload().catch(() => {}); if (alive) paintDetail(); }
      } catch (e) { if (alive) { if (e.status === 404) missing = true; else ctx.toasts.error("No se pudo leer el trabajo", e.message); paintDetail(); } }
    })();
  }
  return () => { alive = false; off(); el.innerHTML = ""; };
}
