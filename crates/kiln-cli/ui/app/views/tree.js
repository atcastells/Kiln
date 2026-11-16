// Árbol: the hierarchy of the game as a collapsible outline with a breadcrumb.
// Only the expanded branches are in the DOM; severity per node is rolled up
// once per catalog (no edges, no per-node scans). Route: #/tree[/node-key]
import { html } from "../html.js";
import { format } from "../router.js";
import { pieceRoute } from "../search.js";
import { SEV_RANK, SEV_LABEL, SEV_SR, plural, dl, findingsBlock, worstOfList, replaceRoute, onPlainClick, postJSON, jobLink } from "./common.js";

const LEVEL = { game: "Juego", place: "Lugar", building: "Edificio", room: "Sala", module: "Módulo", scene: "Escena", asset: "Asset" };
export const levelName = l => LEVEL[l] ?? l;
const STATUS = { model: ["ok", "con modelo"], spec: ["info", "con ficha"], missing: ["error", null] };
const statusOf = n => { const [cls, text] = STATUS[n.status] ?? ["info", n.status]; return [cls, text ?? (n.level === "asset" ? "falta" : "sin definir")]; };

// Findings of each node and the worst severity of each node and everything under it.
export function treeModel(cat, d) {
  const T = cat.tree?.nodes ?? {}, root = cat.tree?.root;
  const byAsset = new Map();
  for (const [subject, list] of d.findingsBySubject) {
    const m = /^(?:asset|part):([^/]+)/.exec(subject);
    if (!m) continue;
    const l = byAsset.get(m[1]);
    if (l) l.push(...list); else byAsset.set(m[1], [...list]);
  }
  const own = new Map();
  for (const n of Object.values(T)) own.set(n.key, n.level === "asset" ? byAsset.get(n.id) ?? [] : d.findingsBySubject.get(n.key) ?? []);
  const worst = new Map(), visiting = new Set();
  const roll = key => {
    if (worst.has(key)) return worst.get(key);
    if (visiting.has(key)) return null;
    visiting.add(key);
    let w = worstOfList(own.get(key));
    for (const c of T[key]?.children ?? []) { const s = T[c] ? roll(c) : null; if ((SEV_RANK[s] ?? 0) > (SEV_RANK[w] ?? 0)) w = s; }
    visiting.delete(key);
    worst.set(key, w);
    return w;
  };
  for (const k of Object.keys(T)) roll(k);
  return { T, root, own, worst };
}

// The rows of the outline: only the children of expanded nodes, never looping on a cycle.
export function flattenOutline(T, root, expanded) {
  const out = [], path = new Set();
  const walk = (key, depth) => {
    const n = T[key];
    if (!n) return;
    path.add(key);
    const kids = n.children.filter(c => T[c] && !path.has(c));
    const open = kids.length > 0 && expanded.has(key);
    out.push({ key, depth, hasKids: kids.length > 0, open });
    if (open) for (const c of kids) walk(c, depth + 1);
    path.delete(key);
  };
  if (root && T[root]) walk(root, 0);
  return out;
}

export function pathTo(T, root, key) {
  const prev = { [root]: null }, queue = [root];
  while (queue.length) {
    const k = queue.shift();
    if (k === key) break;
    for (const c of T[k]?.children ?? []) if (!(c in prev) && T[c]) { prev[c] = k; queue.push(c); }
  }
  if (!(key in prev)) return [];
  const out = [];
  for (let k = key; k; k = prev[k]) out.unshift(k);
  return out;
}

// What the URL names: a node key, `asset:<id>`, or an id.
export function resolveNode(T, arg) {
  if (arg == null) return null;
  if (T[arg]) return arg;
  if (T[`asset:${arg}`]) return `asset:${arg}`;
  return Object.values(T).find(n => n.id === arg)?.key ?? null;
}

export function defaultExpanded(T, root, selected) {
  const set = new Set(root ? [root] : []);
  for (const c of T[root]?.children ?? []) if (T[c]?.children.length) set.add(c);
  if (selected) for (const k of pathTo(T, root, selected)) set.add(k);
  return set;
}

function leavesOf(T, key) {
  const seen = new Set(), out = [], stack = [key];
  while (stack.length) {
    const k = stack.pop();
    if (seen.has(k) || !T[k]) continue;
    seen.add(k);
    const n = T[k];
    if (!n.children.length && k !== key) out.push(n);
    stack.push(...n.children);
  }
  return out;
}

// The request the old tree sent to ask for the spec of a missing asset.
export function askText(T, root, from, id) {
  const path = k => pathTo(T, root, k);
  const holder = from.children.length
    ? path(Object.keys(T).find(k => T[k].children.includes(`asset:${id}`) && path(k).includes(from.key)) ?? from.key)
    : path(from.key).slice(0, -1);
  const where = holder.slice(1).map(k => T[k].name).join(" › ");
  return `crear ${id.replace(/_/g, " ")}${where ? ` para ${where}` : ""}`;
}

export function mount(el, ctx) {
  const { store, api } = ctx;
  const doc = el.ownerDocument, detailEl = doc.getElementById("detail");
  let cat, d, model, expanded, flat = [], first = true;
  const state = { sel: null, active: null };

  const syncUrl = () => replaceRoute({ view: "tree", args: state.sel && state.sel !== model.root ? [state.sel] : [] });

  // ---- panel
  function nodeBody(n) {
    const { T, root, own } = model, fs = own.get(n.key) ?? [];
    const parent = pathTo(T, root, n.key).slice(-2, -1)[0];
    let tail;
    if (n.children.length) {
      const missing = leavesOf(T, n.key).filter(l => l.status !== "model");
      tail = html`<section><h3 class="sec">Lo que falta debajo <span class="n">${missing.length}</span></h3>${missing.length ? html`<ul class="minilist">${missing.slice(0, 40).map(l => html`<li><span><b>${l.name}</b> <span class="muted">${levelName(l.level)} · ${statusOf(l)[1]}</span></span>${l.level === "asset" && l.status === "missing" ? html`<button class="btn sm" type="button" data-ask="${l.id}" data-from="${n.key}">Pedir ficha</button>` : ""}</li>`)}</ul>${missing.length > 40 ? html`<p class="muted small">y ${missing.length - 40} más.</p>` : ""}` : html`<p class="muted">Nada: todo tiene modelo.</p>`}</section>`;
    } else if (n.status === "model") {
      const p = cat.pieces.find(x => x.key === n.piece);
      tail = html`<section><p>Usa la pieza <b>${n.piece}</b> del catálogo.</p>${p ? html`<a class="btn sm" href="${format(pieceRoute(p))}">Ver en el catálogo</a>` : ""}</section>`;
    } else if (n.status === "spec") {
      tail = html`<section><p>Tiene ficha pero aún no modelo.</p><a class="btn sm" href="${format({ view: "spec", args: [n.id] })}">Ver la ficha</a></section>`;
    } else if (n.level === "asset") {
      tail = html`<section><p>Ningún modelo ni ficha. ${parent ? `Lo pide ${levelName(T[parent].level).toLowerCase()} «${T[parent].name}».` : ""}</p><button class="btn sm" type="button" data-ask="${n.id}" data-from="${n.key}">Pedir ficha</button></section>`;
    } else tail = html`<section><p class="muted">Este nodo no está definido: nadie declara sus hijos todavía.</p></section>`;
    return html`<section><h3 class="sec">Datos</h3>${dl([["Id", n.id], ["Declarado en", n.file || "solo como hijo de otro nodo"], ...(n.children.length ? [["Hijos", n.children.length], ["Assets", `${n.modelled} de ${n.assets} con modelo`]] : [])])}</section>${tail}${findingsBlock(fs, "Hallazgos de este nodo")}`;
  }
  function nodeActions(n) {
    if (n.children.length || n.level !== "asset" || n.status !== "missing") return [];
    return [{ label: `Pedir ficha de ${n.name}`, run: () => ask(null, n.id, n.key) }];
  }
  function showNode(key, open) {
    const n = model.T[key];
    if (!n) return;
    const sel = { kind: levelName(n.level), title: n.name, body: nodeBody(n) };
    if (open) ctx.detail.show(sel); else store.set("selection", sel);
    store.set("selectionActions", nodeActions(n));
  }
  async function ask(btn, id, fromKey) {
    const from = model.T[fromKey];
    try {
      const run = () => postJSON(api, "/api/ask", { text: askText(model.T, model.root, from, id) });
      const r = await (btn ? ctx.busy(btn, "Pidiendo la ficha al asistente…", run) : run());
      await ctx.refreshJobs();
      ctx.toasts.ok("Ficha pedida al asistente", `Pedido: ${id.replace(/_/g, " ")}.`, jobLink(ctx, r.job, "Ver el trabajo"));
    } catch (e) { ctx.toasts.error("No se pudo pedir la ficha", `${e.message} Comprueba que el asistente está disponible y vuelve a pulsar Pedir ficha.`); }
  }

  // ---- outline and breadcrumb
  function paintCrumbs() {
    const { T, root } = model;
    const path = pathTo(T, root, state.sel);
    el.querySelector("#tCrumbs").innerHTML = html`<ol class="crumbs">${path.map((k, i) => html`<li>${i === path.length - 1 ? html`<span aria-current="page">${T[k].name}</span>` : html`<a href="${format({ view: "tree", args: [k] })}" data-node="${k}">${T[k].name}</a>`}</li>`)}</ol>`.toString();
  }
  function rowHtml(r, i) {
    const { T, worst } = model, n = T[r.key], sev = worst.get(r.key);
    const [stCls, stText] = statusOf(n);
    const sub = n.name !== n.id ? n.id : (n.piece || n.file || "");
    const right = sev ? html`<span class="sev ${sev}"><span class="dot ${sev}" aria-hidden="true"></span>${SEV_LABEL[sev]}</span>` : !n.children.length ? html`<span class="sev ${stCls}"><span class="dot ${stCls}" aria-hidden="true"></span>${stText}</span>` : "";
    const count = n.children.length ? html`${plural(n.children.length, "hijo", "hijos")} · ${n.modelled} de ${n.assets} con modelo` : "";
    return html`<div class="trow${r.key === state.sel ? " sel" : ""}" role="treeitem" id="tr-${i}" data-key="${r.key}" tabindex="${r.key === state.active ? "0" : "-1"}" aria-level="${r.depth + 1}" aria-selected="${String(r.key === state.sel)}"${r.hasKids ? html` aria-expanded="${String(r.open)}"` : ""} style="--depth:${r.depth}">
      <span class="ttog" data-toggle="${r.key}" aria-hidden="true">${r.hasKids ? (r.open ? "▾" : "▸") : ""}</span><span class="tag">${levelName(n.level)}</span>
      <span class="tname"><b>${n.name}</b>${sub ? html` <span class="muted mono">${sub}</span>` : ""}</span><span class="ct muted">${count}</span><span class="tsev">${right}${sev ? html`<span class="sr"> (${SEV_SR[sev]})</span>` : ""}</span></div>`;
  }
  function paintOutline(focus = false) {
    flat = flattenOutline(model.T, model.root, expanded);
    if (!flat.some(r => r.key === state.active)) state.active = flat.some(r => r.key === state.sel) ? state.sel : model.root;
    const box = el.querySelector("#tOutline");
    box.innerHTML = flat.map(rowHtml).join("");
    el.querySelector("#tCount").textContent = `${plural(flat.length, "nodo visible", "nodos visibles")} de ${Object.keys(model.T).length.toLocaleString("es")}`;
    if (focus) box.querySelector(`[data-key="${CSS.escape(state.active)}"]`)?.focus();
  }
  function select(key, { open = true, reveal = false } = {}) {
    state.sel = state.active = key;
    if (reveal) for (const k of pathTo(model.T, model.root, key).slice(0, -1)) expanded.add(k);
    paintCrumbs(); paintOutline(true); showNode(key, open); syncUrl();
  }
  function toggle(key) {
    if (expanded.has(key)) expanded.delete(key); else expanded.add(key);
    state.active = key; paintOutline(true);
  }

  function draw() {
    cat = store.get("catalog"); d = store.get("derived");
    if (!cat || !d) { el.innerHTML = ""; return; }
    if (!cat.tree || !Object.keys(cat.tree.nodes ?? {}).length) {
      el.innerHTML = html`<div class="page" data-view="tree" data-ready="1"><div class="pagehead"><div><h1>Árbol</h1><div class="sub">Cómo se organiza el juego</div></div></div>
        <div class="empty"><h3>Este proyecto todavía no tiene árbol</h3><p>El árbol sale de las fichas de assets y de los niveles que declara kiln.toml.</p></div></div>`.toString();
      return;
    }
    model = treeModel(cat, d);
    const fromUrl = resolveNode(model.T, ctx.route.args?.[0]);
    if (first) { state.sel = fromUrl ?? model.root; expanded = defaultExpanded(model.T, model.root, state.sel); }
    else if (!model.T[state.sel]) state.sel = model.root;
    state.active = state.sel;
    el.innerHTML = html`<div class="page fill" data-view="tree" data-ready="1">
      <div class="pagehead"><div><h1>Árbol</h1><div class="sub">${plural(Object.keys(model.T).length, "nodo", "nodos")} · ${plural(cat.specs?.length ?? 0, "ficha", "fichas")}</div></div></div>
      <nav id="tCrumbs" aria-label="Ruta hasta el nodo elegido"></nav>
      <div class="card treecard"><div class="tbar"><span id="tCount" class="muted" role="status"></span><span class="spacer"></span><button class="btn sm" type="button" id="tExpand">Expandir todo</button><button class="btn sm" type="button" id="tCollapse">Plegar todo</button></div>
        <div id="tOutline" class="outline" role="tree" aria-label="Árbol del juego"></div></div></div>`.toString();
    paintCrumbs(); paintOutline();
    showNode(state.sel, first && !!fromUrl);
    first = false;
  }

  const offs = [];
  const on = (t, ev, fn) => { t.addEventListener(ev, fn); offs.push(() => t.removeEventListener(ev, fn)); };
  on(el, "click", e => {
    if (e.target.closest("#tExpand")) { expanded = new Set(Object.values(model.T).filter(n => n.children.length).map(n => n.key)); paintOutline(); return; }
    if (e.target.closest("#tCollapse")) { expanded = new Set(); paintOutline(); return; }
    const row = e.target.closest(".trow");
    if (!row || !el.contains(row)) return;
    if (e.target.closest("[data-toggle]")) { toggle(row.dataset.key); return; }
    select(row.dataset.key);
  });
  on(el, "keydown", e => {
    const row = e.target.closest?.(".trow");
    if (!row) return;
    const i = flat.findIndex(r => r.key === row.dataset.key), r = flat[i];
    const go = j => { const t = flat[Math.max(0, Math.min(flat.length - 1, j))]; state.active = t.key; paintOutline(true); };
    switch (e.key) {
      case "ArrowDown": e.preventDefault(); go(i + 1); break;
      case "ArrowUp": e.preventDefault(); go(i - 1); break;
      case "Home": e.preventDefault(); go(0); break;
      case "End": e.preventDefault(); go(flat.length - 1); break;
      case "ArrowRight": e.preventDefault(); if (r.hasKids && !r.open) toggle(r.key); else if (r.open) go(i + 1); break;
      case "ArrowLeft": { e.preventDefault(); if (r.open) toggle(r.key); else { let j = i - 1; while (j >= 0 && flat[j].depth >= r.depth) j--; if (j >= 0) go(j); } break; }
      case "Enter": case " ": e.preventDefault(); select(r.key); break;
      default:
    }
  });
  offs.push(onPlainClick(el, "#tCrumbs a[data-node]", a => select(a.dataset.node, { reveal: true })));
  on(detailEl, "click", e => {
    const b = e.target.closest("button[data-ask]");
    if (b) ask(b, b.dataset.ask, b.dataset.from);
  });
  draw();
  const off = store.subscribe(["catalog", "derived"], draw);
  return () => { off(); offs.forEach(f => f()); ctx.detail.clear(); store.set("selectionActions", null); el.innerHTML = ""; };
}
