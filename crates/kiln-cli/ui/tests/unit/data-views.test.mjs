import test from "node:test";
import assert from "node:assert/strict";
import { derive } from "../../app/derive.js";
import { elementRows, contentFilter, findElement, sortedElements } from "../../app/views/content.js";
import { materialUsage, materialRows, materialFilter } from "../../app/views/materials.js";
import { parseSort, formatSort, highlight, norm } from "../../app/views/common.js";
import { catalog } from "./fixture.mjs";

const cat = catalog(), d = derive(cat);

test("content rows carry their findings by element and a filter by kind and text", () => {
  const rows = elementRows(cat.content[0], d);
  assert.deepEqual(rows.map(r => r.sev), [null, null, "error"]);
  assert.equal(rows[2].findings[0].code, "door");
  assert.deepEqual(rows.filter(contentFilter({ kind: "Sector" })).map(r => r.e.id), ["sector:yard", "sector:hall"]);
  assert.deepEqual(rows.filter(contentFilter({ q: "HALL" })).map(r => r.e.id), ["sector:hall"]);
  assert.equal(contentFilter({}), null);
  assert.deepEqual(d.content.looseBySet.get("world").map(f => f.code), ["set"]);
});

test("findElement takes file#id or a unique id; sorting puts errors first", () => {
  const rows = elementRows(cat.content[0], d);
  assert.equal(findElement(rows, "data/world.ron#door:a").e.id, "door:a");
  assert.equal(findElement(rows, "door:a").e.id, "door:a");
  assert.equal(findElement(rows, "nope"), null);
  const cols = [{ key: "sev", value: r => 3 - ({ error: 3 }[r.sev] ?? 0) }, { key: "id", value: r => r.e.id }];
  assert.equal(sortedElements(rows, null, { key: "sev", dir: 1 }, cols)[0].e.id, "door:a");
  assert.equal(sortedElements(rows, null, { key: "id", dir: -1 }, cols)[0].e.id, "sector:yard");
});

test("materialUsage: a slot uses its bound material or the one named like it", () => {
  const use = materialUsage(cat);
  assert.deepEqual(use.get("pine").map(u => [u.piece.name, u.slot, u.bound]), [["Crate_big", "Wood", true]]);
  assert.deepEqual(use.get("Metal").map(u => [u.piece.name, u.bound]), [["Crate_big", false]]);
  const rows = materialRows(cat, d);
  assert.deepEqual(rows.map(r => r.uses), [1, 1]);
  assert.deepEqual(rows.filter(materialFilter("pino")).map(r => r.m.key), ["pine"]);
  assert.equal(materialFilter(" "), null);
});

test("sort and text helpers", () => {
  assert.deepEqual(parseSort("-tris", ["name", "tris"]), { key: "tris", dir: -1 });
  assert.deepEqual(parseSort("name", ["name"]), { key: "name", dir: 1 });
  assert.equal(parseSort("evil", ["name"]), null);
  assert.equal(parseSort("", ["name"]), null);
  assert.equal(formatSort({ key: "tris", dir: -1 }), "-tris");
  assert.equal(formatSort(null), null);
  assert.equal(norm("Árbol Ñandú"), "arbol nandu");
  assert.equal(highlight("<b>Crate</b>", "crate").toString(), "&lt;b&gt;<mark>Crate</mark>&lt;/b&gt;");
  assert.equal(highlight("x", "").toString(), "x");
});
