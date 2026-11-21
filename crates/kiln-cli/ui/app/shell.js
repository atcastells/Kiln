// The frame of the UI: top bar, navigation, main area, detail panel. It boots
// the data once, keeps the route, and mounts the view of the current route.
import { html } from "./html.js";
import { derive } from "./derive.js";
import { navModel, isRunning } from "./nav.js";
import { createToasts, busy } from "./ui/feedback.js";
import { askText, confirmDialog } from "./ui/dialog.js";
import { createSearch, openPalette } from "./ui/palette.js";
import { indexFor } from "./search.js";

const SEV_TEXT = { error: "con errores", warn: "con avisos", info: "con notas", busy: "en curso" };
const POLL_FIRST = 1500, POLL_MAX = 8000;
// How often the page asks the core what it shows: an MCP session may open
// another project or launch a job at any time.
const WATCH_EVERY = 4000;

export function createShell({ doc = document, store, api, router, registry }) {
  const $ = id => doc.getElementById(id);
  const main = $("main"), shellEl = $("shell"), navEl = $("nav"), detailEl = $("detail");
  const toasts = createToasts($("toastwrap"));
  let unmountView = null, mountToken = 0, pollTimer = null, pollDelay = POLL_FIRST, watchTimer = null, shownRev;

  // ---- data
  function setCatalog(cat) {
    if (!Object.is(store.get("catalog"), cat)) {
      store.set("derived", derive(cat));
      store.set("catalog", cat);
    }
    store.set("loadedAt", Date.now());
  }
  async function loadProject({ fresh = false } = {}) {
    const [cat, context] = await Promise.all([
      api.getJSON("/api/catalog", { fresh }),
      api.getJSON("/api/context").catch(() => null),
    ]);
    store.set("context", context);
    setCatalog(cat);
  }
  // A folder without kiln.toml, or a project whose start is not settled: the wizard.
  const NEW = { view: "new", args: [], query: {} };
  async function leaveWizard() {
    store.set("wizard", null);
    api.forget("/api/new");
    await loadProject();
    refreshJobs();
    router.navigate({ view: "home" });
    store.set("route", router.current());
  }
  async function reload() {
    api.forget("/api/catalog");
    api.forget("/api/context");
    await loadProject({ fresh: true }); // the server drops its cached catalog too
    await refreshJobs();
  }

  // ---- jobs: the `job` key is the running one, `jobs` the list
  async function refreshJobs() {
    let list;
    try { list = (await api.getJSON("/api/jobs")).jobs ?? []; } catch { return; }
    if (JSON.stringify(list) !== JSON.stringify(store.get("jobs"))) store.set("jobs", list);
    const running = list.find(isRunning) ?? null;
    const prev = store.get("job");
    if (running?.id !== prev?.id || running?.label !== prev?.label) store.set("job", running);
    clearTimeout(pollTimer);
    if (running) { pollTimer = setTimeout(refreshJobs, pollDelay); pollDelay = Math.min(POLL_MAX, Math.round(pollDelay * 1.5)); }
    else pollDelay = POLL_FIRST;
  }

  // ---- the core: another project opened from Claude Code reloads the page;
  // a job launched there shows up in the list
  async function watchCore({ first = false } = {}) {
    clearTimeout(watchTimer);
    let s = null;
    try { api.forget("/api/session"); s = await api.getJSON("/api/session"); } catch { /* an older server, or Kiln stopped */ }
    if (s && typeof s.rev === "number") {
      if (shownRev === undefined) { shownRev = s.rev; api.setRev?.(s.rev); }
      else if (s.rev !== shownRev) { doc.defaultView?.location?.reload(); return; }
    }
    if (!first && !doc.hidden && !store.get("job")) await refreshJobs();
    watchTimer = setTimeout(watchCore, WATCH_EVERY);
  }

  // ---- views
  const ctx = {
    store, api, router, toasts, busy,
    get route() { return store.get("route"); },
    navigate: r => router.navigate(r),
    reload, refreshJobs, leaveWizard,
    askText: o => askText(doc, o),
    confirm: o => confirmDialog(doc, o),
    detail: {
      show: sel => { store.set("selection", sel); store.set("panel", true); },
      clear: () => store.set("selection", null),
    },
  };

  async function mountRoute() {
    const route = store.get("route");
    const token = ++mountToken;
    if (unmountView) { try { unmountView(); } catch (e) { console.error(e); } unmountView = null; }
    const load = registry[route.view] ?? registry.home;
    try {
      const mod = await load();
      if (token !== mountToken) return;
      main.innerHTML = "";
      unmountView = mod.mount(main, ctx) ?? null;
      main.setAttribute("aria-busy", "false");
      keepFocus();
      doc.title = `${store.get("catalog")?.project ?? "Kiln"} · Kiln`;
    } catch (e) {
      main.innerHTML = html`<div class="callout" role="alert"><h3>No se pudo abrir esta vista</h3><p>${e.message}</p></div>`.toString();
    }
  }

  // ---- navigation, status, panel
  function drawNav() {
    const cat = store.get("catalog"), derived = store.get("derived");
    shellEl.classList.toggle("in-wizard", !!store.get("wizard"));
    for (const el of [$("gq").parentElement, $("btnReleer"), $("btnPanel")]) el.hidden = !!store.get("wizard");
    if (!cat || !derived || store.get("wizard")) { navEl.innerHTML = ""; return; }
    const current = store.get("route")?.view === "job" ? "jobs" : store.get("route")?.view;
    const groups = navModel({ catalog: cat, derived, context: store.get("context"), jobs: store.get("jobs") ?? [] });
    navEl.innerHTML = groups.map(g => html`${g.title ? html`<h2>${g.title}</h2>` : ""}${g.items.map(i => html`
      <a href="#/${i.view}" data-nav="${i.view}"${i.view === current ? html` aria-current="page"` : ""}><span class="dot ${i.sev ?? "none"}" aria-hidden="true"></span><span>${i.label}${i.sev ? html`<span class="sr"> (${SEV_TEXT[i.sev]})</span>` : ""}</span><span class="cnt">${i.count}</span></a>`)}`).join("")
      + html`<div class="hint"><kbd>Ctrl K</kbd> comandos<br><kbd>/</kbd> buscar<br><a class="classic" href="/classic">Interfaz anterior</a></div>`;
  }
  function drawStatus() {
    const job = store.get("job");
    $("status").innerHTML = (job
      ? html`<a href="#/jobs/${job.id}"><span class="dot busy" aria-hidden="true"></span><span>ocupado: ${job.label}</span></a>`
      : html`<span class="idle"><span class="dot ok" aria-hidden="true"></span><span class="t">Sin trabajos en curso</span></span>`).toString();
  }
  let panelOpener = null, panelWasOpen = false;
  function drawPanel() {
    const open = !!store.get("panel"), sel = store.get("selection");
    if (open && !panelWasOpen && !detailEl.contains(doc.activeElement)) panelOpener = doc.activeElement;
    panelWasOpen = open;
    shellEl.classList.toggle("with-detail", open);
    $("btnPanel").setAttribute("aria-expanded", String(open));
    detailEl.innerHTML = !open ? "" : sel
      ? html`<div class="dhead"><div><div class="kind">${sel.kind}</div><h2>${sel.title}</h2></div><button class="iconbtn" type="button" id="closePanel" aria-label="Cerrar el panel de detalle">×</button></div><div class="dbody">${sel.body}</div>`.toString()
      : html`<div class="dhead"><div><div class="kind">Detalle</div><h2>Nada seleccionado</h2></div><button class="iconbtn" type="button" id="closePanel" aria-label="Cerrar el panel de detalle">×</button></div><div class="dbody"><p class="muted">Elige un elemento en la lista o en el árbol y su detalle aparece aquí.</p></div>`.toString();
  }

  // ---- search box and command palette
  const gq = $("gq"), gres = $("gres");
  createSearch({ doc, input: gq, panel: gres, getIndex: () => indexFor(store.get("catalog"), store.get("context")), go: r => router.navigate(r), onQuery: q => store.set("query", q) });
  const views = () => {
    const cat = store.get("catalog"), derived = store.get("derived");
    if (!cat || !derived) return [];
    return navModel({ catalog: cat, derived, context: store.get("context"), jobs: store.get("jobs") ?? [] }).flatMap(g => g.items).map(i => ({ view: i.view, label: i.label, hint: i.count }));
  };
  function askAsset() {
    router.navigate({ view: "spec", args: [], query: {} });
    let tries = 0;
    const focus = () => { const el = doc.getElementById("sAskText"); if (el) el.focus(); else if (++tries < 20) setTimeout(focus, 50); };
    setTimeout(focus, 50);
  }
  const actions = () => [
    { label: "Releer el proyecto", keywords: "recargar actualizar", run: () => $("btnReleer").click() },
    ...(store.get("selectionActions") ?? []).map(a => ({ group: "Acciones para la selección", ...a })),
    { label: store.get("panel") ? "Cerrar el panel de detalle" : "Abrir el panel de detalle", keywords: "detalle", run: () => store.set("panel", !store.get("panel")) },
    { label: "Pedir un asset…", keywords: "crear nuevo ficha desglosar", run: askAsset },
    { label: "Abrir la interfaz anterior", keywords: "antigua clásica", run: () => { location.href = "/classic"; } },
  ];
  let paletteOpen = false;
  // Focus is never left on <body>: after a view change or a closed dialog it goes to the main area.
  const keepFocus = () => { if (!doc.activeElement || doc.activeElement === doc.body) main.focus({ preventScroll: true }); };
  const commands = async () => {
    if (paletteOpen) return;
    paletteOpen = true;
    try { await openPalette({ doc, views: views(), actions: actions(), getIndex: () => indexFor(store.get("catalog"), store.get("context")), go: r => router.navigate(r) }); }
    finally { paletteOpen = false; setTimeout(keepFocus, 0); }
  };
  $("btnCmd").addEventListener("click", commands);
  doc.addEventListener("keydown", e => {
    const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(doc.activeElement?.tagName ?? "");
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") { e.preventDefault(); commands(); }
    else if (e.key === "Escape" && store.get("panel") && !doc.querySelector("dialog[open]") && gq.getAttribute("aria-expanded") !== "true") { e.preventDefault(); closePanel(); }
    else if (e.key === "/" && !typing && !e.ctrlKey && !e.metaKey && !paletteOpen) { e.preventDefault(); gq.focus(); }
  });
  $("skip").addEventListener("click", () => main.focus());
  $("btnPanel").addEventListener("click", () => store.set("panel", !store.get("panel")));
  function closePanel() {
    const back = panelOpener;
    store.set("panel", false);
    (back?.isConnected && !detailEl.contains(back) ? back : $("btnPanel")).focus();
  }
  detailEl.addEventListener("click", e => { if (e.target.closest("#closePanel")) closePanel(); });
  // below 1100 px the panel is an overlay and the backdrop (the shell's ::after) takes the click
  shellEl.addEventListener("click", e => { if (e.target === shellEl && store.get("panel") && doc.defaultView.matchMedia("(max-width: 1100px)").matches) closePanel(); });
  $("btnReleer").addEventListener("click", async e => {
    try {
      await busy(e.currentTarget, "Releyendo el proyecto desde el disco…", reload);
      toasts.ok("Proyecto releído");
    } catch (err) { toasts.error("No se pudo releer el proyecto", `${err.message} Comprueba que Kiln sigue en marcha y vuelve a pulsar Releer.`); }
  });

  return {
    ctx,
    async start() {
      store.subscribe(["catalog", "derived", "context", "jobs", "route", "wizard"], drawNav);
      store.subscribe("job", drawStatus);
      store.subscribe(["panel", "selection"], drawPanel);
      store.subscribe("catalog", () => { $("kiln-project").textContent = store.get("catalog").project; });
      store.subscribe("route", mountRoute);
      drawStatus(); drawPanel();
      let wz = null;
      try { api.forget("/api/new"); const n = await api.getJSON("/api/new"); wz = n.step ? n : null; } catch { /* not a server with the wizard endpoint */ }
      store.set("wizard", wz);
      await watchCore({ first: true });
      store.set("route", wz ? NEW : router.current());
      router.onChange(r => store.set("route", store.get("wizard") ? NEW : r));
      if (wz) {
        refreshJobs();
        if (wz.step > 1) { try { await loadProject(); } catch { /* the wizard shows what it can */ } }
        return;
      }
      doc.addEventListener("visibilitychange", () => { if (!doc.hidden) refreshJobs(); });
      try {
        await loadProject();
      } catch (e) {
        main.innerHTML = html`<div class="callout" role="alert"><h3>No se pudo cargar el proyecto</h3><p>${e.message}</p><div class="row"><button class="btn primary" type="button" id="retry">Reintentar</button></div></div>`.toString();
        main.setAttribute("aria-busy", "false");
        $("retry").addEventListener("click", () => location.reload());
        return;
      }
      refreshJobs();
    },
  };
}
