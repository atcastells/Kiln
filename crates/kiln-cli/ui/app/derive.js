// Indexes computed once per catalog, each in one pass over its input. Views
// read these instead of filtering `catalog.findings` per row (R4).
const RANK = { info: 1, warn: 2, error: 3 };

export function derive(cat) {
  const findingsBySubject = new Map();
  const worst = new Map();
  const counts = { error: 0, warn: 0, info: 0, total: 0 };
  for (const f of cat.findings) {
    const list = findingsBySubject.get(f.subject);
    if (list) list.push(f); else findingsBySubject.set(f.subject, [f]);
    if ((RANK[f.severity] ?? 0) > (RANK[worst.get(f.subject)] ?? 0)) worst.set(f.subject, f.severity);
    if (f.severity in counts) counts[f.severity]++;
    counts.total++;
  }

  const piecesByKit = new Map();
  for (const k of cat.kits) piecesByKit.set(k.name, []);
  for (const p of cat.pieces) {
    const list = piecesByKit.get(p.kit);
    if (list) list.push(p); else piecesByKit.set(p.kit, [p]);
  }

  // Content: findings by element (subject `<set>:<file>#<id>`), by set (the
  // set itself or `<set>:...`) and the ones of a set that no element owns.
  const bySet = new Map(), looseBySet = new Map(), byElement = new Map();
  const elementKeys = new Set();
  for (const set of cat.content ?? []) {
    bySet.set(set.name, []);
    looseBySet.set(set.name, []);
    for (const e of set.elements) {
      const key = `${set.name}:${e.file}#${e.id}`;
      elementKeys.add(key);
      byElement.set(key, findingsBySubject.get(key) ?? []);
    }
  }
  for (const f of cat.findings) {
    const i = f.subject.indexOf(":");
    const head = i < 0 ? f.subject : f.subject.slice(0, i);
    if (!bySet.has(head)) continue;
    bySet.get(head).push(f);
    if (!elementKeys.has(f.subject)) looseBySet.get(head).push(f);
  }

  // Findings per area of the UI, for the navigation dots and the links of the
  // home view. A subject belongs to the area of what it names.
  const pieceKeys = new Set(cat.pieces.map(p => p.key));
  const kitNames = new Set(cat.kits.map(k => k.name));
  const materialKeys = new Set((cat.materials ?? []).map(m => m.key));
  const areas = { tree: { error: 0, warn: 0, info: 0 }, catalog: { error: 0, warn: 0, info: 0 }, materials: { error: 0, warn: 0, info: 0 }, content: { error: 0, warn: 0, info: 0 }, other: { error: 0, warn: 0, info: 0 } };
  const areaOfSubject = new Map();
  for (const subject of findingsBySubject.keys()) {
    const a = areaOf(subject, { pieceKeys, kitNames, materialKeys, sets: bySet });
    areaOfSubject.set(subject, a);
    for (const f of findingsBySubject.get(subject)) if (f.severity in areas[a]) areas[a][f.severity]++;
  }

  return { findingsBySubject, worst, counts, piecesByKit, areas, areaOfSubject, content: { bySet, looseBySet, byElement } };
}

export function areaOf(subject, { pieceKeys, kitNames, materialKeys, sets }) {
  if (subject.startsWith("part:") || subject.startsWith("asset:")) return "tree";
  const i = subject.indexOf(":");
  if (i > 0 && sets.has(subject.slice(0, i))) return "content";
  if (sets.has(subject)) return "content";
  if (pieceKeys.has(subject) || kitNames.has(subject)) return "catalog";
  if (materialKeys.has(subject)) return "materials";
  return "other";
}

// The worst severity of a `{error, warn, info}` count, or null.
export const worstOf = c => (c?.error ? "error" : c?.warn ? "warn" : c?.info ? "info" : null);

// Where a finding's subject lives in the UI: `{ view, args }`.
export function routeOfSubject(subject, d) {
  const area = d.areaOfSubject.get(subject) ?? "other";
  if (area === "tree") return { view: "tree", args: [subject.replace(/^(part|asset):/, "").split("/")[0]] };
  if (area === "content") {
    const i = subject.indexOf(":");
    return i < 0 ? { view: "content", args: [subject] } : { view: "content", args: [subject.slice(0, i), subject.slice(i + 1)] };
  }
  if (area === "catalog" || area === "materials") return { view: area, args: [subject] };
  return { view: "catalog", args: [] };
}
