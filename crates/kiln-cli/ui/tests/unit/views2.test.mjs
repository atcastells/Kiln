import test from "node:test";
import assert from "node:assert/strict";
import { safeUrl } from "../../app/html.js";
import { fmtN } from "../../app/format.js";
import { schedule, nextDelay, atBottom, nextStep, POLL_MAX } from "../../app/views/jobs.js";
import { wizardStep, stepStates, optionsOf } from "../../app/views/wizard.js";
import { candidateHtml, slug, chosen, partOf, zoneOf, reviewHtml } from "../../app/views/build.js";
import { canValidate, madeOf } from "../../app/views/spec.js";
import { layersOf } from "../../app/views/context.js";

test("safeUrl lets http and https through and makes anything else an inert #", () => {
  assert.equal(safeUrl("https://polyhaven.com/a"), "https://polyhaven.com/a");
  assert.equal(safeUrl("http://x.test/"), "http://x.test/");
  for (const bad of ["javascript:alert(1)", " JaVaScRiPt:alert(1)", "data:text/html,<script>1</script>", "vbscript:x", "//evil.test", "/relative", "", null, undefined, 5]) assert.equal(safeUrl(bad), "#", String(bad));
});

test("a candidate with a javascript: link renders inert, with no thumbnail", () => {
  const out = candidateHtml({ name: "Silla", site: "sketchfab", author: "a", license: "CC0", tris: 10, page_url: "javascript:alert(1)", thumb: "data:image/png;base64,AAAA" }, 0, 100).toString();
  assert.ok(out.includes('href="#"'));
  assert.ok(!out.includes("javascript:") && !out.includes("data:image") && !out.includes("<img"));
  assert.ok(!out.includes('target="_blank"'));
  const ok = candidateHtml({ name: "S", site: "polypizza", author: "a", license: "CC-BY", page_url: "https://poly.pizza/m/1", thumb: "https://t.test/1.png" }, 1, 0).toString();
  assert.ok(ok.includes('href="https://poly.pizza/m/1"') && ok.includes('rel="noopener noreferrer"') && ok.includes("<img"));
  const esc = candidateHtml({ name: "<b>x</b>", site: "polyhaven", author: '"><script>', license: "CC0", page_url: "https://a.test/" }, 0, 0).toString();
  assert.ok(!esc.includes("<script>") && !esc.includes("<b>x"));
});

test("the poll schedule backs off from 1 s to 5 s and stays there", () => {
  assert.deepEqual(schedule(7), [1000, 1500, 2250, 3375, 5000, 5000, 5000]);
  assert.equal(nextDelay(POLL_MAX), POLL_MAX);
});

test("the log follows the end only when the reader is at the bottom", () => {
  assert.equal(atBottom({ scrollTop: 600, scrollHeight: 1000, clientHeight: 400 }), true);
  assert.equal(atBottom({ scrollTop: 590, scrollHeight: 1000, clientHeight: 400 }), true);  // within the slack
  assert.equal(atBottom({ scrollTop: 100, scrollHeight: 1000, clientHeight: 400 }), false);
  assert.equal(atBottom({ scrollTop: 0, scrollHeight: 300, clientHeight: 400 }), true);    // nothing to scroll
});

test("numbers group with a dot, 4 digits included", () => {
  assert.equal(fmtN(5000), "5.000"); assert.equal(fmtN(999), "999"); assert.equal(fmtN(1234567), "1.234.567"); assert.equal(fmtN(0), "0");
  assert.equal(fmtN(null), "—"); assert.equal(fmtN(undefined), "—");
});

test("wizard steps: the server's step, never back from one reached", () => {
  assert.equal(wizardStep({ step: 1 }, 1), 1); assert.equal(wizardStep({ step: 1 }, 2), 2);
  assert.equal(wizardStep({ step: 3 }, 1), 3); assert.equal(wizardStep({ step: 4 }, 5), 5); assert.equal(wizardStep({ step: 5 }, 3), 5);
  assert.deepEqual(stepStates(3).map(s => s.state), ["done", "done", "on", "todo", "todo"]);
  assert.equal(stepStates(1).length, 5);
  assert.deepEqual(optionsOf({ id: "a", options: ["x", "y"] }, { a: ["y", "z"] }), ["x", "y", "z"]);
});

test("what to do next after a job", () => {
  const cat = { builds: { door: { glb: "out/door/door.glb", variants: { red: "out/door/red.glb" } } }, specs: [{ file: "kiln/specs/door.toml", asset: { id: "door" } }] };
  assert.equal(nextStep({ status: "done", spec: "kiln/context.toml" }, cat, { file: "kiln/context.toml" }).target.route.view, "context");
  assert.equal(nextStep({ status: "done", spec: "out/door/review/review.json" }, cat, null).target.route.args[0], "door");
  assert.equal(nextStep({ status: "done", spec: "out/door/red.glb" }, cat, null).target.route.view, "build");
  assert.equal(nextStep({ status: "done", spec: "kiln/specs/door.toml" }, cat, null).target.route.view, "spec");
  const bad = nextStep({ status: "failed" }, cat, null); assert.equal(bad.ok, false); assert.equal(bad.target, null);
});

test("zones, variants and spec helpers", () => {
  assert.equal(partOf("leaf/frame"), "leaf"); assert.equal(zoneOf("leaf/frame"), "frame");
  assert.equal(slug("  Roja Oscura! "), "roja-oscura"); assert.equal(slug("!!"), "");
  const cat = { materials: [{ key: "wood" }, { key: "paint" }], pieces: [{ key: "k/p", bindings: { frame: "wood" } }] };
  const sp = { parts: [{ id: "leaf", source: { from: "k/p", kind: "reuse" }, materials: { frame: "paint" } }], variants: [{ id: "v", materials: { "leaf/frame": "wood" } }] };
  assert.equal(chosen(sp, cat, "leaf/frame", ""), "paint"); assert.equal(chosen(sp, cat, "leaf/frame", "v"), "wood"); assert.equal(chosen(sp, cat, "leaf/other", ""), "");
  assert.equal(canValidate(sp.parts[0], cat), true); assert.equal(canValidate({ ...sp.parts[0], materials: {} }, cat), false);
  assert.deepEqual(madeOf({ parts: [{ source: { kind: "asset", from: "a" } }, { source: { kind: "asset", from: "a" } }, { source: { kind: "reuse", from: "b" } }] }), ["a"]);
  assert.deepEqual(layersOf({ layers: [{ id: "a", name: "A" }], context: { claims: [{ layer: "z" }] } }).map(l => l.id), ["a", "z"]);
});

test("a review escapes what the reviewer wrote", () => {
  const out = reviewHtml({ model: "m", best: 1, rounds: [{ n: 1, dir: "o/review/r1", tris: 5000, review: { verdict: "pass", seen: "<img src=x onerror=1>", scores: { reads: 4 }, findings: [] }, applied: [], left: [] }] }).toString();
  assert.ok(!out.includes("<img src=x") && out.includes("5.000 tri"));
});
