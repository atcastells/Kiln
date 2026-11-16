import test from "node:test";
import assert from "node:assert/strict";
import { buildIndex, searchIndex, flattenGroups, pieceRoute, indexFor } from "../../app/search.js";
import { catalog, piece } from "./fixture.mjs";

const context = { context: { claims: [{ id: "c1", layer: "world", text: "Pueblo de pescadores con una grúa" }] } };
const cat = catalog(), index = buildIndex(cat, context);

test("the index has pieces, kits, materials, elements and claims", () => {
  const by = t => index.filter(i => i.type === t).length;
  assert.deepEqual([by("piece"), by("kit"), by("material"), by("element"), by("claim")], [4, 2, 2, 3, 1]);
});

test("groups come in a fixed order with their totals", () => {
  const g = searchIndex(index, "pine");
  assert.deepEqual(g.map(x => [x.type, x.total]), [["material", 1]]);
  const all = searchIndex(index, "a");
  assert.deepEqual(all.map(x => x.type), ["piece", "kit", "material", "element", "claim"]);
});

test("text matching ignores case and accents", () => {
  assert.equal(searchIndex(index, "ARBOL")[0].items[0].label, "Árbol");
  assert.equal(searchIndex(index, "grua")[0].type, "claim");
  assert.deepEqual(searchIndex(index, "   "), []);
});

test("each group is capped, keeps the true total, and offers the filtered view", () => {
  const big = { ...cat, pieces: Array.from({ length: 40 }, (_, i) => piece("harbor", `post_${String(i).padStart(2, "0")}`)) };
  const [g] = searchIndex(buildIndex(big, null), "post", 5);
  assert.equal(g.total, 40);
  assert.equal(g.items.length, 5);
  assert.equal(g.more, true);
  assert.deepEqual(g.all, { view: "catalog", args: [], query: { q: "post" } });
  const flat = flattenGroups([g]);
  assert.equal(flat.length, 6);
  assert.equal(flat.at(-1).kind, "all");
  assert.match(flat.at(-1).label, /40/);
});

test("names that start with the text rank before ones that only contain it", () => {
  const c = { ...cat, pieces: [piece("a", "wooden_crate"), piece("a", "crate_small"), piece("a", "big_crate")] };
  const [g] = searchIndex(buildIndex(c, null), "crate", 5);
  assert.deepEqual(g.items.map(i => i.label), ["crate_small", "wooden_crate", "big_crate"]);
});

test("pieceRoute drops the kit prefix of the key and the index is memoised per catalog", () => {
  assert.deepEqual(pieceRoute(cat.pieces[0]).args, ["harbor", "beam"]);
  assert.equal(indexFor(cat, context), indexFor(cat, context));
  assert.notEqual(indexFor(cat, context), indexFor({ ...cat }, context));
});
