import test from "node:test";
import assert from "node:assert/strict";
import { derive } from "../../app/derive.js";

const sev = ["info", "warn", "error"];
function catalog(n) {
  const kits = Array.from({ length: Math.max(1, n >> 4) }, (_, i) => ({ name: `kit${i}` }));
  const pieces = Array.from({ length: n }, (_, i) => ({ key: `kit${i % kits.length}/p${i}`, kit: `kit${i % kits.length}` }));
  const sets = [{ name: "world", elements: Array.from({ length: n >> 2 }, (_, i) => ({ id: `e${i}`, file: "f.toml" })) }];
  const findings = Array.from({ length: n }, (_, i) => ({
    severity: sev[i % 3],
    subject: i % 5 === 0 ? `world:f.toml#e${i % (n >> 2 || 1)}` : i % 7 === 0 ? "world" : pieces[i % n].key,
    code: "c", message: "m",
  }));
  return { kits, pieces, content: sets, findings };
}

test("indexes of a small catalog", () => {
  const cat = {
    kits: [{ name: "k1" }, { name: "k2" }],
    pieces: [{ key: "k1/a", kit: "k1" }, { key: "k1/b", kit: "k1" }, { key: "k2/c", kit: "k2" }],
    content: [{ name: "world", elements: [{ id: "e1", file: "w.toml" }, { id: "e2", file: "w.toml" }] }],
    findings: [
      { severity: "warn", subject: "k1/a", message: "1" }, { severity: "error", subject: "k1/a", message: "2" },
      { severity: "info", subject: "k1/b", message: "3" },
      { severity: "warn", subject: "world:w.toml#e1", message: "4" },
      { severity: "info", subject: "world", message: "5" }, { severity: "info", subject: "world:w.toml#zz", message: "6" },
    ],
  };
  const d = derive(cat);
  assert.equal(d.findingsBySubject.get("k1/a").length, 2);
  assert.equal(d.findingsBySubject.get("nothing"), undefined);
  assert.equal(d.worst.get("k1/a"), "error");
  assert.equal(d.worst.get("k1/b"), "info");
  assert.equal(d.worst.get("world:w.toml#e1"), "warn");
  assert.deepEqual(d.piecesByKit.get("k1").map(p => p.key), ["k1/a", "k1/b"]);
  assert.equal(d.piecesByKit.get("k2").length, 1);
  assert.deepEqual(d.counts, { error: 1, warn: 2, info: 3, total: 6 });
  assert.equal(d.content.byElement.get("world:w.toml#e1").length, 1);
  assert.equal(d.content.byElement.get("world:w.toml#e2").length, 0);
  assert.equal(d.content.bySet.get("world").length, 3);
  assert.deepEqual(d.content.looseBySet.get("world").map(f => f.message), ["5", "6"]);
});

test("an empty catalog gives empty indexes", () => {
  const d = derive({ kits: [], pieces: [], content: [], findings: [] });
  assert.equal(d.findingsBySubject.size, 0);
  assert.deepEqual(d.counts, { error: 0, warn: 0, info: 0, total: 0 });
});

// Reads of the catalog arrays are counted through a Proxy: a nested filter per
// row shows up as a quadratic number of reads.
function counted(cat, counter) {
  const wrap = a => new Proxy(a, { get(t, k, r) { counter.n++; return Reflect.get(t, k, r); } });
  return { ...cat, pieces: wrap(cat.pieces), findings: wrap(cat.findings), kits: wrap(cat.kits), content: cat.content.map(c => ({ ...c, elements: wrap(c.elements) })) };
}
const reads = (fn, n) => { const c = { n: 0 }; fn(counted(catalog(n), c)); return c.n; };

test("the number of reads grows linearly with the catalog", () => {
  const small = reads(derive, 2000), big = reads(derive, 4000), huge = reads(derive, 8000);
  assert.ok(small > 2000, "the instrument sees the reads");
  assert.ok(big / small < 2.3, `doubling gave x${(big / small).toFixed(2)}`);
  assert.ok(huge / big < 2.3, `doubling again gave x${(huge / big).toFixed(2)}`);
});

test("the instrument does catch a quadratic implementation", () => {
  const quadratic = cat => cat.pieces.map(p => cat.findings.filter(f => f.subject === p.key).length);
  const ratio = reads(quadratic, 800) / reads(quadratic, 400);
  assert.ok(ratio > 3.5, `x${ratio.toFixed(2)}`);
});
