import { fmtN } from "../format.js";
// Pieces shared by the data views (catalog, materials, content, tree): text
// helpers, the panel blocks and the quiet URL update. Pure unless noted.
import { html, raw } from "../html.js";
import { format } from "../router.js";

export const plural = (n, one, many) => `${fmtN(n)} ${n === 1 ? one : many}`;
export const SEV_LABEL = { error: "Error", warn: "Aviso", info: "Nota" };
export const SEV_RANK = { info: 1, warn: 2, error: 3 };
export const SEV_WORDS = { error: ["error", "errores"], warn: ["aviso", "avisos"], info: ["nota", "notas"] };
export const SEV_SR = { error: "con errores", warn: "con avisos", info: "con notas" };

// Lower case without accents, for matching.
export const norm = s => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

export const fmtSize = v => (v ? v.map(n => n.toFixed(2)).join(" × ") + " m" : "sin medir");

export const worstOfList = list => {
  let best = null;
  for (const f of list ?? []) if ((SEV_RANK[f.severity] ?? 0) > (SEV_RANK[best] ?? 0)) best = f.severity;
  return best;
};

// `text` with the first match of `q` in <mark>; escaped either way.
export function highlight(text, q) {
  const s = String(text ?? ""), n = norm(q).trim();
  if (!n) return html`${s}`;
  const t = norm(s);
  const at = t.length === s.length ? t.indexOf(n) : -1;
  if (at < 0) return html`${s}`;
  return html`${s.slice(0, at)}<mark>${s.slice(at, at + n.length)}</mark>${s.slice(at + n.length)}`;
}

// The status of a row: shape + word (never colour alone).
export const sevBadge = sev => (sev
  ? html`<span class="sev ${sev}"><span class="dot ${sev}" aria-hidden="true"></span>${SEV_LABEL[sev]}</span>`
  : html`<span class="sev ok"><span class="dot ok" aria-hidden="true"></span>Sin problemas</span>`);

export const sevDot = sev => html`<span class="dot ${sev ?? "none"}" aria-hidden="true"></span>${sev ? html`<span class="sr">(${SEV_SR[sev]})</span>` : ""}`;

export const dl = pairs => html`<dl>${pairs.filter(([, v]) => v != null && v !== "").map(([k, v]) => html`<dt>${k}</dt><dd>${v}</dd>`)}</dl>`;

export function findingsBlock(list, title = "Hallazgos de este elemento") {
  if (!list?.length) return html`<section><h3 class="sec">${title}</h3><p class="muted">Sin hallazgos.</p></section>`;
  return html`<section><h3 class="sec">${title} <span class="n">${list.length}</span></h3>${list.map(f => html`
    <div class="finding"><div class="fh"><span class="sev ${f.severity}"><span class="dot ${f.severity}" aria-hidden="true"></span>${SEV_LABEL[f.severity] ?? f.severity}</span><code>${f.code}</code></div><div>${f.message}</div></div>`)}</section>`;
}

// ---- URL: a view keeps its own state in the hash without remounting
export function replaceRoute(route, win = globalThis.window) {
  try { win?.history?.replaceState(null, "", format(route)); } catch { /* no history: the state just stays in memory */ }
}

// `sort=name` ascending, `sort=-tris` descending; `keys` are the sortable columns.
export function parseSort(text, keys) {
  if (!text) return null;
  const dir = text.startsWith("-") ? -1 : 1, key = text.replace(/^-/, "");
  return keys.includes(key) ? { key, dir } : null;
}
export const formatSort = s => (s ? (s.dir === -1 ? "-" : "") + s.key : null);

// A link click that the view handles itself (plain left click only).
export function onPlainClick(el, selector, fn) {
  const h = e => {
    if (e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
    const a = e.target.closest(selector);
    if (a && el.contains(a)) { e.preventDefault(); fn(a, e); }
  };
  el.addEventListener("click", h);
  return () => el.removeEventListener("click", h);
}

// POST that treats `{error}` in a 200 body as a failure, as the old page did.
export async function postJSON(api, path, body) {
  const r = await api.post(path, body);
  if (r?.error) throw new Error(r.error);
  return r;
}

export const jobLink = (ctx, job, label) => ({ label, run: () => ctx.navigate({ view: "jobs", args: [String(job)] }) });
export { html, raw };
export { fmtN };
