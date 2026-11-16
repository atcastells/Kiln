import test from "node:test";
import assert from "node:assert/strict";
import { parse, format, createRouter, VIEWS } from "../../app/router.js";

test("parse reads view, args and query", () => {
  assert.deepEqual(parse("#/catalog/kit01/piece%20a?q=hi&sort=name"), { view: "catalog", args: ["kit01", "piece a"], query: { q: "hi", sort: "name" } });
  assert.deepEqual(parse("#/tree"), { view: "tree", args: [], query: {} });
  assert.deepEqual(parse("#/tree/?flag"), { view: "tree", args: [], query: { flag: "" } });
});

test("empty, malformed and unknown routes go home", () => {
  const home = { view: "home", args: [], query: {} };
  for (const h of ["", "#", "#/", "#/nope/x", "#/%E0%A4%A", "nonsense"]) assert.deepEqual(parse(h), home, h);
});

test("format and parse round-trip, with encoding", () => {
  const r = { view: "spec", args: ["a/b", "c d", "é"], query: { q: "x&y=z", n: "1" } };
  const h = format(r);
  assert.ok(h.startsWith("#/spec/"));
  assert.ok(!h.slice(2).includes(" "));
  assert.deepEqual(parse(h), r);
  assert.equal(format({ view: "home" }), "#/home");
  assert.equal(format({ view: "tree", args: [], query: { a: null, b: undefined, c: "1" } }), "#/tree?c=1");
});

test("every known view round-trips", () => {
  for (const v of VIEWS) assert.equal(parse(format({ view: v, args: ["x"], query: {} })).view, v);
});

const fakeWindow = () => {
  const ls = new Set();
  const w = {
    location: { _h: "", get hash() { return this._h; }, set hash(v) { this._h = v.startsWith("#") ? v : "#" + v; w.fire(); } },
    addEventListener: (t, f) => { if (t === "hashchange") ls.add(f); },
    removeEventListener: (t, f) => ls.delete(f),
    fire: () => [...ls].forEach(f => f()),
    listeners: ls,
  };
  return w;
};

test("navigate sets the hash and onChange hears it; back and forward are hash changes", () => {
  const w = fakeWindow(), r = createRouter({ win: w });
  const seen = [];
  const off = r.onChange(route => seen.push(route));
  r.navigate({ view: "tree", args: ["root"] });
  r.navigate({ view: "catalog", args: [], query: { q: "a" } });
  w.location._h = "#/tree/root"; w.fire(); // the browser going back
  assert.deepEqual(seen.map(s => s.view), ["tree", "catalog", "tree"]);
  assert.deepEqual(seen[2].args, ["root"]);
  assert.deepEqual(r.current(), { view: "tree", args: ["root"], query: {} });
  off();
  assert.equal(w.listeners.size, 0);
  r.navigate({ view: "home" });
  assert.equal(seen.length, 3);
});

test("onChange with immediate reports the current route first", () => {
  const w = fakeWindow(); w.location._h = "#/content";
  const seen = [];
  createRouter({ win: w }).onChange(r => seen.push(r.view), { immediate: true });
  assert.deepEqual(seen, ["content"]);
});
