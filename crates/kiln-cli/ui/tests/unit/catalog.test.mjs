import test from "node:test";
import assert from "node:assert/strict";
import { derive } from "../../app/derive.js";
import { catalogFilter, pieceRows, resolveArgs, sevCounts, sortedRows } from "../../app/views/catalog.js";
import { catalog } from "./fixture.mjs";

const cat = catalog(), d = derive(cat), rows = pieceRows(cat, d);
const names = rs => rs.map(r => r.p.name);

test("pieceRows carries the worst severity and a normalised haystack", () => {
  assert.deepEqual(rows.map(r => r.sev), ["warn", "error", null, null]);
  assert.ok(rows[3].hay.includes("arbol"), "accents are stripped");
  assert.ok(rows[1].hay.includes("pine"), "the bound material is searchable");
});

test("catalogFilter composes kit, text and severity", () => {
  assert.equal(catalogFilter({}), null);
  assert.deepEqual(names(rows.filter(catalogFilter({ kit: "harbor" }))), ["beam", "Crate_big", "rope"]);
  assert.deepEqual(names(rows.filter(catalogFilter({ q: "CRATE" }))), ["Crate_big"]);
  assert.deepEqual(names(rows.filter(catalogFilter({ sev: "error" }))), ["Crate_big"]);
  assert.deepEqual(names(rows.filter(catalogFilter({ sev: "ok" }))), ["rope", "Árbol"]);
  assert.deepEqual(names(rows.filter(catalogFilter({ kit: "harbor", sev: "ok", q: "ro" }))), ["rope"]);
  assert.deepEqual(names(rows.filter(catalogFilter({ q: "arbol" }))), ["Árbol"]);
});

test("sortedRows filters then sorts, descending and with missing values last", () => {
  const byTris = sortedRows(rows, catalogFilter({ kit: "harbor" }), { key: "tris", dir: 1 });
  assert.deepEqual(names(byTris), ["beam", "Crate_big", "rope"]);
  assert.deepEqual(names(sortedRows(rows, catalogFilter({ kit: "harbor" }), { key: "tris", dir: -1 })), ["Crate_big", "beam", "rope"]);
  assert.deepEqual(names(sortedRows(rows, null, { key: "sev", dir: 1 }))[0], "Crate_big", "worst first");
  assert.deepEqual(names(sortedRows(rows, null, null)), names(rows), "no sort keeps the catalog order");
});

test("resolveArgs reads a kit, kit + rest of key, or a whole key", () => {
  assert.deepEqual(resolveArgs([], cat), { kit: "", piece: null });
  assert.equal(resolveArgs(["harbor"], cat).kit, "harbor");
  assert.equal(resolveArgs(["harbor", "beam"], cat).piece.key, "harbor/beam");
  assert.equal(resolveArgs(["harbor", "harbor/beam"], cat).piece.key, "harbor/beam");
  const whole = resolveArgs(["town/Árbol"], cat);
  assert.equal(whole.piece.name, "Árbol");
  assert.equal(whole.kit, "town");
  assert.deepEqual(resolveArgs(["nope"], cat), { kit: "", piece: null });
});

test("sevCounts counts pieces without findings as ok", () => {
  assert.deepEqual(sevCounts(rows), { error: 1, warn: 1, info: 0, ok: 2 });
});
