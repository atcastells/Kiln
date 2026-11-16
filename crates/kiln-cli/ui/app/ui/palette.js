import { fmtN } from "../format.js";
// The top-bar search (debounced, grouped results) and the command palette
// (Ctrl/Cmd+K, on the dialog component). The matching is pure and tested in Node.
import { html } from "../html.js";
import { openDialog } from "./dialog.js";
import { searchIndex, flattenGroups } from "../search.js";
import { norm, highlight } from "../views/common.js";

const wordStart = (hay, at) => (at === 0 ? 0 : /[^a-z0-9]/.test(hay[at - 1]) ? 1 : 2);

// Commands that contain the text, the ones that start with it first; all of them when it is empty.
export function matchCommands(commands, query) {
  const n = norm(query).trim();
  if (!n) return commands.slice();
  const out = [];
  commands.forEach((c, i) => {
    const hay = norm(`${c.label} ${c.keywords ?? ""}`), at = hay.indexOf(n);
    if (at >= 0) out.push({ c, score: wordStart(hay, at), i });
  });
  return out.sort((a, b) => a.score - b.score || a.i - b.i).map(x => x.c);
}

// What the palette lists: views, actions, and (once something is typed) pieces, kits, materials and elements.
export function paletteModel({ q = "", views = [], actions = [], index = [], go, cap = 5 }) {
  const viewCmds = views.map(v => ({ group: "Ir a", label: v.label, hint: v.hint ?? "", run: () => go({ view: v.view, args: [], query: {} }) }));
  const out = [...matchCommands(viewCmds, q), ...matchCommands(actions.map(a => ({ group: "Acciones", ...a })), q)];
  if (norm(q).trim()) {
    for (const g of searchIndex(index, q, cap)) {
      if (g.type === "claim") continue;
      for (const it of g.items) out.push({ group: g.title, label: it.label, hint: it.hint, mono: g.type === "piece", run: () => go(it.route) });
    }
  }
  return out;
}

// ---- command palette
export async function openPalette({ doc, views, actions, getIndex, go }) {
  const chosen = await openDialog(doc, html`<div class="pal">
      <input class="field" type="text" data-focus role="combobox" aria-expanded="true" aria-controls="palList" aria-autocomplete="list" autocomplete="off"
             aria-label="Comando: escribe una vista, una acción o el nombre de una pieza" placeholder="Ir a una vista, una pieza o ejecutar una acción">
      <div class="list" id="palList" role="listbox" aria-label="Comandos"></div>
      <div class="foot"><span><kbd>↑</kbd> <kbd>↓</kbd> moverse</span><span><kbd>Enter</kbd> ejecutar</span><span><kbd>Esc</kbd> cerrar</span></div></div>`.toString(), {
    onReady(dlg, finish) {
      dlg.id = "dlgPal";
      dlg.setAttribute("aria-label", "Paleta de comandos");
      const input = dlg.querySelector("input"), list = dlg.querySelector(".list");
      let items = [], active = 0;
      const paint = () => {
        const q = input.value;
        items = paletteModel({ q, views, actions, index: getIndex(), go });
        if (active >= items.length) active = Math.max(0, items.length - 1);
        let last = null;
        list.innerHTML = items.length ? items.map((c, i) => {
          const head = c.group !== last ? html`<h3 role="presentation">${c.group}</h3>` : "";
          last = c.group;
          return html`${head}<div class="opt" role="option" id="pal-${i}" data-i="${i}" aria-selected="${String(i === active)}"><span class="t${c.mono ? " mono" : ""}">${highlight(c.label, q)}</span><span class="h">${c.hint ?? ""}</span></div>`;
        }).join("") : html`<div class="sfoot" role="status">Nada coincide con «${q}». Prueba con otra palabra.</div>`.toString();
        if (items.length) input.setAttribute("aria-activedescendant", `pal-${active}`); else input.removeAttribute("aria-activedescendant");
        list.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
      };
      input.addEventListener("input", () => { active = 0; paint(); });
      input.addEventListener("keydown", e => {
        if (e.key === "ArrowDown") { e.preventDefault(); active = Math.min(items.length - 1, active + 1); paint(); }
        else if (e.key === "ArrowUp") { e.preventDefault(); active = Math.max(0, active - 1); paint(); }
        else if (e.key === "Home") { e.preventDefault(); active = 0; paint(); }
        else if (e.key === "End") { e.preventDefault(); active = Math.max(0, items.length - 1); paint(); }
        else if (e.key === "Enter") { e.preventDefault(); if (items[active]) finish(items[active]); }
      });
      list.addEventListener("click", e => { const o = e.target.closest(".opt"); if (o) finish(items[Number(o.dataset.i)]); });
      paint();
    },
  });
  if (chosen) await chosen.run();
}

// ---- top-bar search
export function createSearch({ doc, input, panel, getIndex, go, onQuery = () => {}, delay = 120, setTimer = setTimeout, clearTimer = clearTimeout }) {
  let opts = [], active = -1, timer = null;
  const close = () => { panel.hidden = true; input.setAttribute("aria-expanded", "false"); input.removeAttribute("aria-activedescendant"); active = -1; };
  const mark = () => {
    panel.querySelectorAll(".opt").forEach((o, i) => o.setAttribute("aria-selected", String(i === active)));
    if (active >= 0) { input.setAttribute("aria-activedescendant", `sr-${active}`); panel.querySelector(`#sr-${active}`)?.scrollIntoView({ block: "nearest" }); } else input.removeAttribute("aria-activedescendant");
  };
  function paint(q) {
    const groups = searchIndex(getIndex(), q, 5);
    opts = flattenGroups(groups);
    let i = 0;
    panel.innerHTML = (groups.length ? groups.map(g => html`<div role="group" aria-label="${g.title}"><h3 role="presentation">${g.title} <span class="n">${fmtN(g.total)}</span></h3>${g.items.map(it => {
      const n = i++;
      return html`<div class="opt" role="option" id="sr-${n}" data-i="${n}" aria-selected="false"><span class="t${g.type === "piece" ? " mono" : ""}">${highlight(it.label, q)}</span><span class="h">${it.hint}</span></div>`;
    })}${g.more ? (() => { const n = i++; return html`<div class="opt all" role="option" id="sr-${n}" data-i="${n}" aria-selected="false"><span class="t">Ver todos (${fmtN(g.total)})</span><span class="h">en ${g.title}</span></div>`; })() : ""}</div>`).join("")
      : html`<div class="sfoot" role="status">Nada coincide con «${q}». Prueba con otra palabra.</div>`.toString()) + (groups.length ? html`<div class="sfoot">↑ ↓ para moverte, Enter para abrir, Esc para cerrar</div>`.toString() : "");
    panel.hidden = false;
    input.setAttribute("aria-expanded", "true");
    active = -1;
  }
  function pick(i) {
    const o = opts[i];
    if (!o) return;
    close();
    input.value = "";
    onQuery("");
    go(o.route);
    doc.getElementById("main")?.focus();
  }
  input.addEventListener("input", () => {
    clearTimer(timer);
    timer = setTimer(() => {
      const q = input.value.trim();
      onQuery(q);
      if (!q) close(); else paint(q);
    }, delay);
  });
  input.addEventListener("keydown", e => {
    const open = !panel.hidden && opts.length > 0;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (!open) return;
      e.preventDefault();
      active = e.key === "ArrowDown" ? Math.min(opts.length - 1, active + 1) : Math.max(0, active - 1);
      mark();
    } else if (e.key === "Enter") {
      if (!open) return;
      e.preventDefault();
      clearTimer(timer);
      pick(active >= 0 ? active : 0);
    } else if (e.key === "Escape") {
      clearTimer(timer);
      if (!panel.hidden) close(); else { input.value = ""; onQuery(""); }
    }
  });
  panel.addEventListener("mousedown", e => e.preventDefault());
  panel.addEventListener("click", e => { const o = e.target.closest(".opt"); if (o) pick(Number(o.dataset.i)); });
  doc.addEventListener("click", e => { if (!panel.hidden && e.target !== input && !panel.contains(e.target)) close(); });
  return { close, paint };
}
