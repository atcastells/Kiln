import test from "node:test";
import assert from "node:assert/strict";
import { specToml } from "../../app/views/spec.js";

const spec = { asset: { id: "rock", name: "Roca", asked: "una roca", used_by: ["mapa"], where: "", states: [], budget_tris: 5000, status: "brief", notes: "", animations: [] }, parts: [], generator: { seed: 1, ops: [] } };
const recipe = { seed: 7, material: "rock", ops: [{ op: "sphere", radius_m: [8, 40], segments: 64 }, { op: "facet" }] };

test("emits asset and generator, skipping empty fields", () => {
  assert.equal(specToml(spec, recipe), [
    "[asset]", 'id = "rock"', 'name = "Roca"', 'asked = "una roca"', 'used_by = ["mapa"]', "budget_tris = 5000", 'status = "brief"',
    "", "[generator]", "seed = 7", 'material = "rock"',
    "", "[[generator.ops]]", 'op = "sphere"', "radius_m = [8, 40]", "segments = 64",
    "", "[[generator.ops]]", 'op = "facet"', "",
  ].join("\n"));
});

test("never emits parts", () => assert.ok(!specToml(spec, recipe).includes("[[parts]]")));

test("escapes quotes, backslashes and newlines in strings", () => {
  const s = { ...spec, asset: { ...spec.asset, name: 'Roca "grande"\nC:\\x' } };
  assert.ok(specToml(s, recipe).includes('name = "Roca \\"grande\\"\\nC:\\\\x"'));
});

test("omits material when the recipe has none", () => assert.ok(!specToml(spec, { seed: 3, ops: [] }).includes("material")));

test("keeps the anchors of the spec", () => {
  const text = specToml({ asset: { id: "rock", name: "Roca" }, anchors: [{ id: "top", class: "socket", at: [0, 1.5, 0], turn: [0, 90, 0] }] }, { seed: 1, ops: [{ op: "sphere", radius_m: 2, segments: 16 }] });
  assert.match(text, /\[\[anchors\]\]\nid = "top"\nclass = "socket"\nat = \[0, 1\.5, 0\]\nturn = \[0, 90, 0\]/);
});
