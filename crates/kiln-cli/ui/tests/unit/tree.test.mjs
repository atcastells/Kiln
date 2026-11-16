import test from "node:test";
import assert from "node:assert/strict";
import { derive } from "../../app/derive.js";
import { treeModel, flattenOutline, pathTo, resolveNode, defaultExpanded, askText } from "../../app/views/tree.js";
import { catalog } from "./fixture.mjs";

const cat = catalog(), d = derive(cat), { T, root } = treeModel(cat, d);

test("flattenOutline lists only the children of expanded nodes", () => {
  assert.deepEqual(flattenOutline(T, root, new Set()).map(r => r.key), ["game:game"]);
  assert.deepEqual(flattenOutline(T, root, new Set([root])).map(r => r.key), ["game:game", "place:port", "asset:lamp"]);
  const open = flattenOutline(T, root, new Set([root, "place:port"]));
  assert.deepEqual(open.map(r => [r.key, r.depth]), [["game:game", 0], ["place:port", 1], ["asset:crane", 2], ["asset:ghost", 2], ["asset:lamp", 1]]);
  assert.deepEqual(open.map(r => r.open), [true, true, false, false, false]);
  assert.equal(open[2].hasKids, false);
});

test("flattenOutline survives a cycle", () => {
  const loop = { a: { key: "a", children: ["b"] }, b: { key: "b", children: ["a"] } };
  assert.deepEqual(flattenOutline(loop, "a", new Set(["a", "b"])).map(r => r.key), ["a", "b"]);
});

test("severity rolls up from the findings of an asset and its parts to every ancestor", () => {
  assert.equal(treeModel(cat, d).worst.get("asset:crane"), "info");
  assert.equal(treeModel(cat, d).worst.get("place:port"), "info");
  assert.equal(treeModel(cat, d).worst.get("game:game"), "info");
  assert.equal(treeModel(cat, d).worst.get("asset:lamp"), null);
  assert.equal(treeModel(cat, d).own.get("asset:crane").length, 1);
});

test("pathTo, resolveNode and defaultExpanded", () => {
  assert.deepEqual(pathTo(T, root, "asset:ghost"), ["game:game", "place:port", "asset:ghost"]);
  assert.deepEqual(pathTo(T, root, "nope"), []);
  assert.equal(resolveNode(T, "asset:crane"), "asset:crane");
  assert.equal(resolveNode(T, "crane"), "asset:crane");
  assert.equal(resolveNode(T, "port"), "place:port");
  assert.equal(resolveNode(T, "zzz"), null);
  assert.deepEqual([...defaultExpanded(T, root, "asset:ghost")].sort(), ["game:game", "asset:ghost", "place:port"].sort());
});

test("askText names the asset and where it is asked for", () => {
  assert.equal(askText(T, root, T["place:port"], "ghost"), "crear ghost para Puerto");
  assert.equal(askText(T, root, T["asset:ghost"], "ghost"), "crear ghost para Puerto");
});
