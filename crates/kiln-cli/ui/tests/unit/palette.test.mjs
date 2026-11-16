import test from "node:test";
import assert from "node:assert/strict";
import { matchCommands, paletteModel, createSearch } from "../../app/ui/palette.js";
import { buildIndex } from "../../app/search.js";
import { catalog } from "./fixture.mjs";

const views = [{ view: "home", label: "Inicio" }, { view: "catalog", label: "Catálogo", hint: "4" }, { view: "materials", label: "Materiales" }];
const index = buildIndex(catalog(), null);

test("matchCommands: empty keeps all, text filters ignoring accents, prefixes first", () => {
  const cmds = [{ label: "Ver catálogo" }, { label: "Catálogo" }, { label: "Inicio" }];
  assert.equal(matchCommands(cmds, "").length, 3);
  assert.deepEqual(matchCommands(cmds, "catalogo").map(c => c.label), ["Catálogo", "Ver catálogo"]);
  assert.deepEqual(matchCommands(cmds, "zzz"), []);
  assert.deepEqual(matchCommands([{ label: "Releer", keywords: "recargar" }], "recar").length, 1);
});

test("paletteModel: views and actions with nothing typed; pieces once something is", () => {
  const went = [];
  const m0 = paletteModel({ views, actions: [{ label: "Releer el proyecto", run() {} }], index, go: r => went.push(r) });
  assert.deepEqual(m0.map(c => [c.group, c.label]), [["Ir a", "Inicio"], ["Ir a", "Catálogo"], ["Ir a", "Materiales"], ["Acciones", "Releer el proyecto"]]);
  const m1 = paletteModel({ q: "crate", views, actions: [], index, go: r => went.push(r) });
  assert.deepEqual(m1.map(c => [c.group, c.label]), [["Piezas", "Crate_big"]]);
  m1[0].run();
  assert.deepEqual(went.at(-1).args, ["harbor", "Crate_big"]);
  const m2 = paletteModel({ q: "cat", views, actions: [], index, go: () => {} });
  assert.equal(m2[0].label, "Catálogo");
});

// A minimal page: just what createSearch touches.
function fake() {
  const listeners = {};
  const el = () => ({ listeners: {}, attrs: {}, hidden: true, value: "", innerHTML: "", addEventListener(t, f) { this.listeners[t] = f; }, setAttribute(k, v) { this.attrs[k] = v; }, removeAttribute(k) { delete this.attrs[k]; }, querySelectorAll: () => [], querySelector: () => null, contains: () => false });
  const input = el(), panel = el();
  const doc = { addEventListener(t, f) { listeners[t] = f; }, getElementById: () => ({ focus() {} }) };
  return { input, panel, doc };
}

test("createSearch waits 120 ms of quiet before searching, once for a burst of keys", () => {
  const { input, panel, doc } = fake();
  const timers = []; let calls = 0; const queries = [];
  createSearch({ doc, input, panel, getIndex: () => { calls++; return index; }, go() {}, onQuery: q => queries.push(q), setTimer: (f, ms) => { timers.push({ f, ms }); return timers.length; }, clearTimer: id => { if (id) timers[id - 1].dead = true; } });
  for (const v of ["c", "cr", "cra"]) { input.value = v; input.listeners.input(); }
  assert.equal(calls, 0);
  assert.ok(timers.every(t => t.ms === 120));
  timers.filter(t => !t.dead).forEach(t => t.f());
  assert.equal(calls, 1);
  assert.deepEqual(queries, ["cra"]);
  assert.equal(panel.hidden, false);
  assert.match(panel.innerHTML, /<mark>Cra<\/mark>te_big/);
});

test("createSearch: Enter opens the first result, Escape closes, an empty query closes", () => {
  const { input, panel, doc } = fake();
  const timers = [], went = [];
  createSearch({ doc, input, panel, getIndex: () => index, go: r => went.push(r), setTimer: f => { timers.push(f); return timers.length; }, clearTimer() {} });
  input.value = "crate"; input.listeners.input(); timers.at(-1)();
  const key = k => { let prevented = false; input.listeners.keydown({ key: k, preventDefault() { prevented = true; } }); return prevented; };
  assert.equal(key("ArrowDown"), true);
  assert.equal(key("Enter"), true);
  assert.deepEqual(went[0].args, ["harbor", "Crate_big"]);
  assert.equal(panel.hidden, true);
  input.value = "zzzz"; input.listeners.input(); timers.at(-1)();
  assert.match(panel.innerHTML, /Nada coincide/);
  assert.equal(key("Escape"), false);
  assert.equal(panel.hidden, true);
  input.value = ""; input.listeners.input(); timers.at(-1)();
  assert.equal(panel.hidden, true);
});
