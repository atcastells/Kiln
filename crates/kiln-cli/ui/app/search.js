// The global search over the derived project data. Pure: the index is built
// once per catalog, a query walks it once and keeps only the best few per group.
import { norm } from "./views/common.js";

export const GROUPS = [
  { type: "piece", title: "Piezas", all: q => ({ view: "catalog", args: [], query: { q } }) },
  { type: "kit", title: "Kits", all: () => ({ view: "catalog", args: [], query: {} }) },
  { type: "material", title: "Materiales", all: q => ({ view: "materials", args: [], query: { q } }) },
  { type: "element", title: "Contenido", all: q => ({ view: "content", args: [], query: { q } }) },
  { type: "claim", title: "Contexto", all: q => ({ view: "context", args: [], query: { q } }) },
];

// The route of a piece: `#/catalog/<kit>/<rest of the key>` when the key starts with the kit.
export const pieceRoute = p => ({ view: "catalog", args: [p.kit, p.key.startsWith(p.kit + "/") ? p.key.slice(p.kit.length + 1) : p.key], query: {} });

export function buildIndex(cat, context) {
  const out = [];
  const add = (type, label, hint, hay, route) => out.push({ type, label, hint, hay: norm(hay), route });
  for (const p of cat.pieces) add("piece", p.name, p.kit, `${p.name} ${p.key}`, pieceRoute(p));
  for (const k of cat.kits) add("kit", k.name, `${k.pieces} piezas`, k.name, { view: "catalog", args: [k.name], query: {} });
  for (const m of cat.materials ?? []) add("material", m.key, m.name !== m.key ? m.name : "", `${m.key} ${m.name}`, { view: "materials", args: [m.key], query: {} });
  for (const s of cat.content ?? []) for (const e of s.elements) add("element", e.id, `${e.kind} · ${s.name}`, `${e.id} ${e.kind} ${e.file}`, { view: "content", args: [s.name, `${e.file}#${e.id}`], query: {} });
  for (const c of context?.context?.claims ?? []) add("claim", c.text, c.layer, c.text, { view: "context", args: [], query: {} });
  return out;
}

let memo = { cat: null, context: null, index: null };
export function indexFor(cat, context) {
  if (!cat) return [];
  if (memo.cat !== cat || memo.context !== context) memo = { cat, context, index: buildIndex(cat, context) };
  return memo.index;
}

// 0 = the name starts with it, 1 = a word does, 2 = it is somewhere inside.
const scoreOf = (hay, at) => (at === 0 ? 0 : /[^a-z0-9]/.test(hay[at - 1]) ? 1 : 2);

// Groups with `total` matches and the best `cap` of them, in the order of GROUPS.
export function searchIndex(index, query, cap = 5) {
  const n = norm(query).trim();
  if (!n) return [];
  const by = new Map(GROUPS.map(g => [g.type, { ...g, total: 0, items: [] }]));
  let seq = 0;
  for (const it of index) {
    const at = it.hay.indexOf(n);
    if (at < 0) continue;
    const g = by.get(it.type);
    g.total++;
    const score = scoreOf(it.hay, at), list = g.items;
    if (list.length >= cap && score >= list[list.length - 1].score) continue;
    list.push({ ...it, score, seq: seq++ });
    list.sort((a, b) => a.score - b.score || a.seq - b.seq);
    if (list.length > cap) list.pop();
  }
  return [...by.values()].filter(g => g.total > 0).map(g => ({ type: g.type, title: g.title, total: g.total, items: g.items, all: g.all(query.trim()), more: g.total > g.items.length }));
}

// The options of the keyboard list: each result, then "ver todos" when the group has more.
export function flattenGroups(groups) {
  const out = [];
  for (const g of groups) {
    for (const it of g.items) out.push({ kind: "item", group: g.title, label: it.label, hint: it.hint, route: it.route });
    if (g.more) out.push({ kind: "all", group: g.title, label: `Ver todos (${g.total.toLocaleString("es")})`, route: g.all });
  }
  return out;
}
