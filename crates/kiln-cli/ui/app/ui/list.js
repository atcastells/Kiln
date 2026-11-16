import { fmtN } from "../format.js";
// A virtualised list/table: fixed row height, only the visible window (plus
// a small overscan) is in the DOM. The helpers up here are pure and tested in
// Node; `createList` below is the DOM part.
import { html } from "../html.js";

export const ROW_HEIGHT = 34;
export const OVERSCAN = 6;

// The rows to render: [start, end) and the offset of the first one.
export function windowRange({ scrollTop, viewport, rowHeight = ROW_HEIGHT, total, overscan = OVERSCAN }) {
  if (total <= 0 || viewport <= 0) return { start: 0, end: 0, offset: 0 };
  const first = Math.min(total - 1, Math.max(0, Math.floor(scrollTop / rowHeight)));
  const last = Math.min(total, Math.ceil((scrollTop + viewport) / rowHeight));
  const start = Math.max(0, first - overscan), end = Math.min(total, last + overscan);
  return { start, end, offset: start * rowHeight };
}

const collator = new Intl.Collator("es", { numeric: true, sensitivity: "base" });
const cmp = (a, b) => {
  if (a == null && b == null) return 0;
  if (a == null) return 1;
  if (b == null) return -1;
  return typeof a === "number" && typeof b === "number" ? a - b : collator.compare(String(a), String(b));
};

// A sorted copy; `dir` is 1 or -1. Stable. Rows with no value go last.
export function sortRows(rows, value, dir = 1) {
  if (!value) return rows.slice();
  return rows.map((r, i) => [r, i, value(r)]).sort((x, y) => {
    const nullx = x[2] == null, nully = y[2] == null;
    if (nullx !== nully) return nullx ? 1 : -1;
    return dir * cmp(x[2], y[2]) || x[1] - y[1];
  }).map(x => x[0]);
}

export const filterRows = (rows, fn) => (fn ? rows.filter(fn) : rows);

// The active row after a key; `null` when the key is not for navigation.
export function moveActive(index, key, total, page = 10) {
  if (total <= 0) return -1;
  const at = index < 0 ? 0 : index;
  switch (key) {
    case "ArrowDown": return index < 0 ? 0 : Math.min(total - 1, at + 1);
    case "ArrowUp": return Math.max(0, at - 1);
    case "PageDown": return Math.min(total - 1, at + page);
    case "PageUp": return Math.max(0, at - page);
    case "Home": return 0;
    case "End": return total - 1;
    default: return null;
  }
}

export const nextSort = (sort, key) => (sort?.key === key ? (sort.dir === 1 ? { key, dir: -1 } : null) : { key, dir: 1 });
export const counterText = (shown, total) => (shown === total ? `${fmtN(total)} filas` : `${fmtN(shown)} de ${fmtN(total)}`);

let seq = 0;

// columns: [{ key, label, width ("1fr", "120px"), align: "r", mono, sortable,
//   value(row) -> sortable value, render(row) -> html | text }]
export function createList(host, { columns, rows = [], rowId = (r, i) => i, rowHeight = ROW_HEIGHT, filter = null, sort = null, onSelect = () => {}, label = "Lista", empty = "No hay filas." }) {
  const id = `vl${++seq}`;
  let all = rows, shown = rows, activeIdx = -1, selectedId = null, view = { start: -1, end: -1 };
  let sorting = sort, predicate = filter;
  host.innerHTML = html`<div class="vgrid" role="grid" aria-label="${label}" tabindex="0" style="--cols:${columns.map(c => c.width ?? "1fr").join(" ")}">
    <div class="vh" role="row">${columns.map(c => html`<div role="columnheader" class="${c.align === "r" ? "r" : ""}" data-key="${c.key}" aria-sort="none">${c.sortable === false ? html`<span class="hbtn">${c.label}</span>` : html`<button class="hbtn" type="button" data-key="${c.key}">${c.label}<span class="arr" aria-hidden="true"></span></button>`}</div>`)}</div>
    <div class="vb"><div class="vs"><div class="vr" role="rowgroup"></div></div><div class="vnone empty small" hidden><p>${empty}</p></div></div>
    <div class="vfoot"><span class="vcount" role="status"></span></div></div>`.toString();
  const grid = host.querySelector(".vgrid"), body = host.querySelector(".vb"), space = host.querySelector(".vs"), rowsEl = host.querySelector(".vr");
  const none = host.querySelector(".vnone"), countEl = host.querySelector(".vcount");

  const key = (r, i) => String(rowId(r, i));
  const paint = force => {
    const r = windowRange({ scrollTop: body.scrollTop, viewport: body.clientHeight, rowHeight, total: shown.length });
    if (!force && r.start === view.start && r.end === view.end) return;
    view = r;
    rowsEl.style.transform = `translateY(${r.offset}px)`;
    let out = "";
    for (let i = r.start; i < r.end; i++) {
      const row = shown[i], k = key(row, i);
      out += html`<div class="vrow${k === selectedId ? " sel" : ""}${i === activeIdx ? " act" : ""}" role="row" id="${id}-r${i}" data-i="${i}" aria-rowindex="${i + 1}" aria-selected="${String(k === selectedId)}" style="height:${rowHeight}px">${columns.map(c => html`<div role="gridcell" class="vc${c.align === "r" ? " r" : ""}${c.mono ? " mono" : ""}">${c.render ? c.render(row) : row[c.key]}</div>`)}</div>`;
    }
    rowsEl.innerHTML = out;
  };
  const headers = () => {
    for (const h of host.querySelectorAll(".vh [role=columnheader]")) {
      const on = sorting?.key === h.dataset.key;
      h.setAttribute("aria-sort", on ? (sorting.dir === 1 ? "ascending" : "descending") : "none");
      const b = h.querySelector("button");
      if (b) { b.toggleAttribute("data-on", on); if (on) b.dataset.dir = String(sorting.dir); else delete b.dataset.dir; }
    }
  };
  const recompute = () => {
    const col = sorting && columns.find(c => c.key === sorting.key);
    shown = sortRows(filterRows(all, predicate), col && (col.value ?? (r => r[col.key])), sorting?.dir ?? 1);
    space.style.height = `${shown.length * rowHeight}px`;
    none.hidden = shown.length > 0;
    countEl.textContent = counterText(shown.length, all.length);
    grid.setAttribute("aria-rowcount", String(shown.length));
    if (activeIdx >= shown.length) activeIdx = shown.length - 1;
    headers();
    paint(true);
  };
  const reveal = i => {
    const top = i * rowHeight, bottom = top + rowHeight;
    if (top < body.scrollTop) body.scrollTop = top;
    else if (bottom > body.scrollTop + body.clientHeight) body.scrollTop = bottom - body.clientHeight;
  };
  const setActive = i => {
    activeIdx = i;
    if (i >= 0) { reveal(i); grid.setAttribute("aria-activedescendant", `${id}-r${i}`); } else grid.removeAttribute("aria-activedescendant");
    paint(true);
  };
  const select = i => {
    if (i < 0 || i >= shown.length) return;
    selectedId = key(shown[i], i);
    setActive(i);
    onSelect(shown[i], i);
  };

  body.addEventListener("scroll", () => paint(false), { passive: true });
  host.querySelector(".vh").addEventListener("click", e => {
    const b = e.target.closest("button[data-key]");
    if (b) { sorting = nextSort(sorting, b.dataset.key); body.scrollTop = 0; recompute(); }
  });
  // mousedown, not click: focusing the grid repaints the rows, and a click needs the same node at press and release.
  rowsEl.addEventListener("mousedown", e => {
    const r = e.button === 0 && e.target.closest(".vrow");
    if (r) { e.preventDefault(); grid.focus(); select(Number(r.dataset.i)); }
  });
  grid.addEventListener("keydown", e => {
    if (e.key === "Enter") { if (activeIdx >= 0) { e.preventDefault(); select(activeIdx); } return; }
    const next = moveActive(activeIdx, e.key, shown.length, Math.max(1, Math.floor(body.clientHeight / rowHeight) - 1));
    if (next == null) return;
    e.preventDefault();
    setActive(next);
  });
  grid.addEventListener("focus", () => { if (activeIdx < 0 && shown.length) setActive(0); });
  const ro = typeof ResizeObserver === "function" ? new ResizeObserver(() => paint(true)) : null;
  ro?.observe(body);

  recompute();
  return {
    setRows(next) { all = next; recompute(); },
    setFilter(fn) { predicate = fn; body.scrollTop = 0; recompute(); },
    setSort(next) { sorting = next; recompute(); },
    select, count: () => shown.length, total: () => all.length,
    destroy() { ro?.disconnect(); host.innerHTML = ""; },
  };
}
