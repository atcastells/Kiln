import test from "node:test";
import assert from "node:assert/strict";
import { derive, routeOfSubject } from "../../app/derive.js";
import { navModel } from "../../app/nav.js";
import { nextStep, isEmptyProject } from "../../app/views/home.js";

const f = (severity, subject, message = "m") => ({ severity, subject, code: "c", message });
function cat(over = {}) {
  return {
    project: "p", kits: [{ name: "kit1", path: "a/kit1" }], materials: [{ key: "mat1" }],
    pieces: [{ key: "kit1/a", kit: "kit1" }, { key: "kit1/b", kit: "kit1" }],
    tree: { nodes: { x: {}, y: {}, z: {} } }, specs: [], content: [], findings: [], ...over,
  };
}
const items = g => g.flatMap(x => x.items);
const by = (g, v) => items(g).find(i => i.view === v);

test("nav counters come from the catalog; content only when there are sets", () => {
  const c = cat();
  const g = navModel({ catalog: c, derived: derive(c), context: { context: { questions: [1, 2], claims: [1] } }, jobs: [] });
  assert.deepEqual(items(g).map(i => i.view), ["home", "context", "tree", "catalog", "materials", "jobs"]);
  assert.equal(by(g, "catalog").count, "2");
  assert.equal(by(g, "tree").count, "3");
  assert.equal(by(g, "materials").count, "1");
  assert.equal(by(g, "context").count, "2 abiertas");
  assert.equal(by(g, "jobs").count, "");
  const c2 = cat({ content: [{ name: "world", kinds: [], elements: [{ file: "w", id: "a" }, { file: "w", id: "b" }] }] });
  const g2 = navModel({ catalog: c2, derived: derive(c2), context: null, jobs: [] });
  assert.equal(by(g2, "content").count, "2");
});

test("nav severity dots take the worst finding of each area", () => {
  const c = cat({
    findings: [f("warn", "kit1/a"), f("error", "kit1/b"), f("info", "part:x/y"), f("warn", "mat1"), f("error", "world:w#a")],
    content: [{ name: "world", kinds: [], elements: [{ file: "w", id: "a" }] }],
  });
  const g = navModel({ catalog: c, derived: derive(c), context: null, jobs: [] });
  assert.equal(by(g, "catalog").sev, "error");
  assert.equal(by(g, "tree").sev, "info");
  assert.equal(by(g, "materials").sev, "warn");
  assert.equal(by(g, "content").sev, "error");
});

test("running jobs show in the Trabajos entry", () => {
  const c = cat();
  const g = navModel({ catalog: c, derived: derive(c), context: null, jobs: [{ id: 1, state: "running" }, { id: 2, state: "done" }] });
  assert.equal(by(g, "jobs").count, "1 en curso");
  assert.equal(by(g, "jobs").sev, "busy");
});

test("routeOfSubject sends each finding to its element", () => {
  const c = cat({ content: [{ name: "world", kinds: [], elements: [] }] });
  const d = derive(Object.assign(c, { findings: [f("error", "part:cargo/hoist"), f("error", "kit1/a"), f("error", "world:data/w.ron#door:a"), f("warn", "mat1"), f("warn", "???")] }));
  assert.deepEqual(routeOfSubject("part:cargo/hoist", d), { view: "tree", args: ["cargo"] });
  assert.deepEqual(routeOfSubject("kit1/a", d), { view: "catalog", args: ["kit1/a"] });
  assert.deepEqual(routeOfSubject("world:data/w.ron#door:a", d), { view: "content", args: ["world", "data/w.ron#door:a"] });
  assert.deepEqual(routeOfSubject("mat1", d), { view: "materials", args: ["mat1"] });
  assert.deepEqual(routeOfSubject("???", d), { view: "catalog", args: [] });
});

test("next step: errors first, then warnings, then no kits, then all good", () => {
  const step = c => nextStep(c, derive(c));
  const e = cat({ findings: [f("warn", "kit1/a"), f("error", "kit1/b", "demasiados triángulos")] });
  assert.equal(step(e).key, "errors");
  assert.match(step(e).text, /demasiados triángulos/);
  assert.deepEqual(step(e).actions[0].route, { view: "catalog", args: ["kit1/b"] });
  assert.equal(step(cat({ findings: [f("warn", "kit1/a"), f("info", "kit1/b")] })).key, "warnings");
  assert.equal(step(cat({ kits: [], pieces: [] })).key, "no-kits");
  assert.equal(step(cat({ findings: [f("info", "kit1/a")] })).key, "ok");
});

test("Astra-like project (no kits, one content set) is not an empty project", () => {
  const c = cat({ kits: [], pieces: [], materials: [], tree: { nodes: { g: {} } }, content: [{ name: "world", kinds: [{ name: "Door", count: 1 }], elements: [{ file: "w", id: "a" }] }] });
  assert.equal(isEmptyProject(c), false);
  const s = nextStep(c, derive(c));
  assert.equal(s.key, "ok");
  assert.equal(s.actions[0].route.view, "content");
  assert.equal(isEmptyProject(cat({ kits: [], pieces: [], materials: [], tree: { nodes: { g: {} } } })), true);
});
