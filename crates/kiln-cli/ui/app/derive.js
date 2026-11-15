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

  return { findingsBySubject, worst, counts, piecesByKit, content: { bySet, looseBySet, byElement } };
}
