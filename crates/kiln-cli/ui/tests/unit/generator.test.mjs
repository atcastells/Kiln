import test from "node:test";
import assert from "node:assert/strict";
import { controlsOf, recipeOf, setSeed, clamp } from "../../app/views/generator.js";

const P = (id, min, max, def, integer = false) => ({ id, label: id.toUpperCase(), unit: "", min, max, default: def, integer });
const vocab = [
  { id: "sphere", label: "Esfera", repeatable: false, params: [{ ...P("radius_m", 0.1, 10000, 1), unit: "m" }, P("segments", 8, 256, 64, true)] },
  { id: "stretch", label: "Estirar", repeatable: false, params: [P("x", 0.2, 5, 1), P("y", 0.2, 5, 1), P("z", 0.2, 5, 1)] },
  { id: "noise", label: "Ruido", repeatable: true, params: [P("amplitude", 0, 1, 0.3), P("size", 0.01, 4, 1)] },
];
const recipe = () => ({ seed: 7, material: "rock", ops: [
  { op: "sphere", radius_m: [8, 40], segments: 64 },
  { op: "noise", amplitude: [0.2, 0.5], size: 0.6 },
  { op: "stretch", x: 2 },
  { op: "warp", power: 3 },
] });

test("groups follow the recipe order with vocabulary labels", () => {
  const g = controlsOf(recipe(), vocab);
  assert.deepEqual(g.map(x => [x.index, x.op, x.label, x.known]), [[0, "sphere", "Esfera", true], [1, "noise", "Ruido", true], [2, "stretch", "Estirar", true], [3, "warp", "warp", false]]);
});

test("range and fixed params", () => {
  const [sphere] = controlsOf(recipe(), vocab);
  const [r, s] = sphere.controls;
  assert.equal(r.range, true); assert.deepEqual(r.value, [8, 40]); assert.equal(r.unit, "m");
  assert.equal(s.range, false); assert.equal(s.value, 64); assert.equal(s.step, 1); assert.equal(s.integer, true);
  assert.equal(r.step, "any");
});

test("absent param shows the default as fixed", () => {
  const st = controlsOf(recipe(), vocab)[2];
  assert.deepEqual(st.controls.map(c => [c.param, c.value, c.range]), [["x", 2, false], ["y", 1, false], ["z", 1, false]]);
});

test("unknown op is not known and has no controls", () => {
  const w = controlsOf(recipe(), vocab)[3];
  assert.equal(w.known, false); assert.deepEqual(w.controls, []);
});

test("recipeOf applies only the edit and does not mutate", () => {
  const r = recipe(), before = JSON.stringify(r);
  const out = recipeOf(r, [{ index: 1, param: "size", value: [0.5, 1] }, { index: 2, param: "y", value: 3 }]);
  assert.equal(JSON.stringify(r), before);
  const want = recipe(); want.ops[1].size = [0.5, 1]; want.ops[2].y = 3;
  assert.deepEqual(out, want);
  assert.deepEqual(Object.keys(out.ops[1]), ["op", "amplitude", "size"]);
});

test("setSeed coerces to u32", () => {
  assert.equal(setSeed(recipe(), "42.9").seed, 42);
  assert.equal(setSeed(recipe(), -1).seed, 0);
  assert.equal(setSeed(recipe(), 5e12).seed, 4294967295);
  assert.equal(setSeed(recipe(), "abc").seed, 0);
  assert.equal(setSeed(recipe(), 9).material, "rock");
});

test("clamp rounds, limits and orders", () => {
  const seg = { min: 8, max: 256, integer: true }, amp = { min: 0, max: 1, integer: false };
  assert.equal(clamp(seg, 63.6), 64); assert.equal(clamp(seg, 1), 8); assert.equal(clamp(seg, 900), 256);
  assert.equal(clamp(amp, 1.5), 1); assert.equal(clamp(amp, 0.25), 0.25);
  assert.deepEqual(clamp(amp, [0.9, 0.1]), [0.1, 0.9]);
  assert.deepEqual(clamp(seg, [300, 2.4]), [8, 256]);
});
