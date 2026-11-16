import { fmtN } from "../format.js";
// Inicio: what the project has, what fails (by severity), the next step and
// the jobs. Everything comes from the store; nothing is fetched here.
import { html, raw } from "../html.js";
import { format } from "../router.js";
import { routeOfSubject } from "../derive.js";
import { isRunning } from "../nav.js";

const href = r => format(r);
const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;
const SEV = {
  error: { dot: "error", one: "error", many: "errores", title: "errores" },
  warn: { dot: "warn", one: "aviso", many: "avisos", title: "avisos" },
  info: { dot: "info", one: "nota", many: "notas", title: "notas" },
};
const TOP = 4;

// The suggested next step, by simple rules: errors, then warnings, then an
// empty project, then "all good". Pure; the view only draws what it returns.
export function nextStep(cat, d) {
  const { error, warn } = d.counts;
  const first = sev => cat.findings.find(f => f.severity === sev);
  const sets = cat.content ?? [];
  if (error > 0) {
    const f = first("error");
    return { key: "errors", title: error === 1 ? "Corrige el error del proyecto" : `Corrige los ${error} errores del proyecto`,
      text: `Bloquean el trabajo. Empieza por el primero: ${f.message}`,
      actions: [{ label: "Abrir el primer error", route: routeOfSubject(f.subject, d), primary: true }, { label: error === 1 ? "Ver el error" : `Ver los ${error} errores`, route: { view: "catalog", args: [], query: { severity: "error" } } }] };
  }
  if (warn > 0) {
    const f = first("warn");
    return { key: "warnings", title: warn === 1 ? "Revisa el aviso del proyecto" : `Revisa los ${warn} avisos del proyecto`,
      text: `No bloquean, pero conviene mirarlos: ${f.message}`,
      actions: [{ label: "Abrir el primer aviso", route: routeOfSubject(f.subject, d), primary: true }, { label: warn === 1 ? "Ver el aviso" : `Ver los ${warn} avisos`, route: { view: "catalog", args: [], query: { severity: "warn" } } }] };
  }
  if (cat.kits.length === 0 && sets.length === 0) {
    return { key: "no-kits", title: "Todavía no hay kits en el proyecto",
      text: "Importa un kit de assets o pide una pieza al asistente; con eso el catálogo empieza a llenarse.",
      actions: [{ label: "Importar o pedir una pieza", href: "/", primary: true }] };
  }
  const onlyContent = cat.kits.length === 0;
  return { key: "ok", title: "Todo en orden",
    text: onlyContent ? "El contenido está validado y no hay hallazgos pendientes." : "No hay errores ni avisos. Puedes seguir construyendo.",
    actions: [onlyContent ? { label: "Ver el contenido", route: { view: "content", args: [] }, primary: true } : { label: "Ver el catálogo", route: { view: "catalog", args: [] }, primary: true }] };
}

export const isEmptyProject = cat =>
  cat.kits.length === 0 && cat.pieces.length === 0 && (cat.specs ?? []).length === 0 && (cat.content ?? []).length === 0
  && Object.keys(cat.tree?.nodes ?? {}).length <= 1;

const clock = secs => new Date(secs * 1000).toLocaleTimeString("es", { hour: "2-digit", minute: "2-digit" });
const jobState = { running: ["busy", "en curso"], done: ["ok", "terminado"], failed: ["error", "falló"] };

function tiles(cat, ctx) {
  const sets = cat.content ?? [];
  const elements = sets.reduce((n, s) => n + s.elements.length, 0);
  const kinds = new Set(sets.flatMap(s => s.kinds.map(k => k.name))).size;
  const layers = ctx?.layers?.length ?? 0, claims = ctx?.context?.claims?.length ?? 0;
  const list = [
    { v: cat.kits.length, l: "Kits", d: `${plural(new Set(cat.kits.map(k => k.path.split("/").slice(0, -1).join("/"))).size, "lugar", "lugares")}`, r: { view: "catalog" } },
    { v: cat.pieces.length, l: "Piezas", d: `en ${plural(cat.kits.length, "kit", "kits")}`, r: { view: "catalog" } },
    { v: (cat.materials ?? []).length, l: "Materiales", d: "del proyecto", r: { view: "materials" } },
  ];
  if (sets.length) list.push({ v: elements, l: "Elementos", d: `${plural(sets.length, "conjunto", "conjuntos")} · ${plural(kinds, "tipo", "tipos")}`, r: { view: "content" } });
  list.push({ v: layers, l: "Capas de contexto", d: plural(claims, "afirmación", "afirmaciones"), r: { view: "context" } });
  return html`<div class="tiles" style="--n:${list.length}">${list.map(t => html`
    <a class="tile${t.v ? "" : " zero"}" href="${href({ view: t.r.view })}"><div class="v">${fmtN(t.v)}</div><div class="l">${t.l}</div><div class="d">${t.d}</div></a>`)}</div>`;
}

function contentSets(cat, d) {
  const sets = cat.content ?? [];
  if (!sets.length) return "";
  return html`<section aria-labelledby="h-content"><h2 class="sec" id="h-content">Contenido <span class="n">${sets.length}</span></h2>
    <div class="card issues"><ul>${sets.map(s => {
      const f = d.content.bySet.get(s.name) ?? [];
      const e = f.filter(x => x.severity === "error").length, w = f.filter(x => x.severity === "warn").length;
      const st = e ? html`<span class="sev error"><span class="dot error"></span>${plural(e, "error", "errores")}</span>`
        : w ? html`<span class="sev warn"><span class="dot warn"></span>${plural(w, "aviso", "avisos")}</span>`
        : html`<span class="sev ok"><span class="dot ok"></span>validado</span>`;
      return html`<li><a class="set-row" href="${href({ view: "content", args: [s.name] })}"><span><b>${s.name}</b> <span class="muted">· ${plural(s.elements.length, "elemento", "elementos")}, ${s.kinds.map(k => `${k.count} ${k.name}`).join(", ")}</span></span>${st}</a></li>`;
    })}</ul></div></section>`;
}

function findings(cat, d) {
  const { counts } = d;
  if (counts.error + counts.warn === 0) {
    return html`<section aria-labelledby="h-fail"><h2 class="sec" id="h-fail">Qué falla</h2>
      <div class="calm"><span class="dot ok" aria-hidden="true"></span><div><b>Sin errores ni avisos</b><span>${counts.info ? `${plural(counts.info, "nota", "notas")} informativas.` : "El proyecto no tiene hallazgos."}</span></div></div></section>`;
  }
  const by = { error: [], warn: [], info: [] };
  for (const f of cat.findings) if (by[f.severity] && by[f.severity].length < TOP) by[f.severity].push(f);
  return html`<section aria-labelledby="h-fail"><h2 class="sec" id="h-fail">Qué falla <span class="n">${plural(counts.total, "hallazgo", "hallazgos")}</span></h2>
    <div class="card issues">${["error", "warn", "info"].filter(s => counts[s]).map(s => html`
      <div class="gh"><span class="dot ${SEV[s].dot}" aria-hidden="true"></span><span class="t">${plural(counts[s], SEV[s].one, SEV[s].many)}</span><a href="${href({ view: "catalog", args: [], query: { severity: s } })}">Ver todos</a></div>
      <ul>${by[s].map(f => html`<li><a class="issue" href="${href(routeOfSubject(f.subject, d))}"><span class="mono">${f.subject}</span><span class="m">${f.message}</span><span class="w">${f.code}</span></a></li>`)}</ul>`)}
    </div></section>`;
}

function nextCard(step) {
  return html`<section class="card next" aria-labelledby="h-next" data-next="${step.key}"><div class="sec" id="h-next">Siguiente paso</div><h3>${step.title}</h3><p>${step.text}</p>
    <div class="row">${step.actions.map(a => html`<a class="btn${a.primary ? " primary" : ""}" href="${a.href ?? href(a.route)}">${a.label}</a>`)}</div></section>`;
}

function jobsCard(jobs) {
  const running = jobs.filter(isRunning);
  const body = !jobs.length
    ? html`<div class="empty small"><p>Todavía no se ha lanzado ningún trabajo.</p></div>`
    : html`<ul class="joblist">${jobs.slice().reverse().slice(0, 6).map(j => {
      const [dot, label] = jobState[j.state] ?? ["info", j.state];
      return html`<li><a href="${href({ view: "jobs", args: [String(j.id)] })}"><span class="dot ${dot}" aria-hidden="true"></span><span>${j.label}</span><span class="when">${label}${j.started ? ` · ${clock(j.started)}` : ""}</span></a></li>`;
    })}</ul>`;
  return html`<section aria-labelledby="h-jobs"><h2 class="sec" id="h-jobs">Trabajos <span class="n">${running.length ? `${running.length} en curso` : ""}</span></h2><div class="card">${body}</div></section>`;
}

function emptyProject() {
  return html`<div class="empty" data-empty="project"><h3>Este proyecto todavía está vacío</h3>
    <p>No hay kits, piezas ni contenido. Tres pasos para empezar:</p>
    <ol><li>Escribe en el contexto qué juego es y cómo se ve.</li><li>Importa un kit de assets o pide una pieza al asistente.</li><li>Vuelve aquí: Kiln te dirá qué corregir y qué hacer a continuación.</li></ol>
    <div class="row"><a class="btn primary" href="${href({ view: "context" })}">Ir al contexto</a> <a class="btn" href="/">Importar o pedir una pieza</a></div></div>`;
}

export function mount(el, ctx) {
  const { store } = ctx;
  function draw() {
    const cat = store.get("catalog"), d = store.get("derived");
    if (!cat || !d) { el.innerHTML = ""; return; }
    const context = store.get("context"), jobs = store.get("jobs") ?? [], loaded = store.get("loadedAt");
    const empty = isEmptyProject(cat);
    el.innerHTML = html`<div class="page" data-view="home" data-ready="1">
      <div class="pagehead"><div><h1>${cat.project}</h1><div class="sub">${loaded ? `Releído a las ${clock(loaded / 1000)}` : ""}</div></div></div>
      ${empty ? emptyProject() : raw("")}
      <div class="home-grid"><div>
        <section aria-labelledby="h-has"><h2 class="sec" id="h-has">Qué tiene el proyecto</h2>${tiles(cat, context)}</section>
        ${contentSets(cat, d)}
        ${empty ? "" : findings(cat, d)}
      </div><div>${nextCard(nextStep(cat, d))}${jobsCard(jobs)}</div></div>
    </div>`.toString();
  }
  draw();
  const off = store.subscribe(["catalog", "derived", "context", "jobs", "loadedAt"], draw);
  return () => { off(); el.innerHTML = ""; };
}
