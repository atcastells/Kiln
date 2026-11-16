import test from "node:test";
import assert from "node:assert/strict";
import { windowRange, sortRows, filterRows, moveActive, nextSort, counterText } from "../../app/ui/list.js";

test("windowRange renders only the visible rows plus the overscan", () => {
  const r = windowRange({ scrollTop: 0, viewport: 340, rowHeight: 34, total: 5000, overscan: 6 });
  assert.deepEqual(r, { start: 0, end: 16, offset: 0 });
  const mid = windowRange({ scrollTop: 34 * 1000, viewport: 340, rowHeight: 34, total: 5000, overscan: 6 });
  assert.equal(mid.start, 994);
  assert.equal(mid.end, 1016);
  assert.equal(mid.offset, 994 * 34);
  assert.ok(mid.end - mid.start < 80);
});

test("windowRange clamps at the end and handles empty or zero-height lists", () => {
  const end = windowRange({ scrollTop: 34 * 4990, viewport: 340, rowHeight: 34, total: 5000 });
  assert.equal(end.end, 5000);
  assert.deepEqual(windowRange({ scrollTop: 0, viewport: 340, total: 0 }), { start: 0, end: 0, offset: 0 });
  assert.deepEqual(windowRange({ scrollTop: 0, viewport: 0, total: 10 }), { start: 0, end: 0, offset: 0 });
  assert.equal(windowRange({ scrollTop: 1e9, viewport: 340, total: 20 }).end, 20);
});

test("sortRows sorts numbers and text, both ways, stable, nulls last", () => {
  const rows = [{ n: "b2", v: 3 }, { n: "a10", v: 1 }, { n: "a9", v: 1 }, { n: "c", v: null }];
  assert.deepEqual(sortRows(rows, r => r.v, 1).map(r => r.n), ["a10", "a9", "b2", "c"]);
  assert.deepEqual(sortRows(rows, r => r.v, -1).map(r => r.n), ["b2", "a10", "a9", "c"]);
  assert.deepEqual(sortRows(rows, r => r.n, 1).map(r => r.n), ["a9", "a10", "b2", "c"]);
  assert.deepEqual(rows.map(r => r.n), ["b2", "a10", "a9", "c"], "input untouched");
});

test("sortRows on 5,000 rows is fast", () => {
  const rows = Array.from({ length: 5000 }, (_, i) => ({ k: `piece${(i * 7919) % 5000}` }));
  const t = performance.now();
  sortRows(rows, r => r.k, 1);
  assert.ok(performance.now() - t < 500);
});

test("filterRows keeps matches; no function keeps all", () => {
  const rows = [1, 2, 3, 4];
  assert.deepEqual(filterRows(rows, n => n % 2 === 0), [2, 4]);
  assert.equal(filterRows(rows, null), rows);
});

test("moveActive: arrows, paging, Home/End, clamped; other keys are not navigation", () => {
  assert.equal(moveActive(-1, "ArrowDown", 5), 0);
  assert.equal(moveActive(2, "ArrowDown", 5), 3);
  assert.equal(moveActive(4, "ArrowDown", 5), 4);
  assert.equal(moveActive(0, "ArrowUp", 5), 0);
  assert.equal(moveActive(3, "ArrowUp", 5), 2);
  assert.equal(moveActive(3, "Home", 5), 0);
  assert.equal(moveActive(0, "End", 5), 4);
  assert.equal(moveActive(0, "PageDown", 100, 10), 10);
  assert.equal(moveActive(5, "PageUp", 100, 10), 0);
  assert.equal(moveActive(0, "ArrowDown", 0), -1);
  assert.equal(moveActive(0, "a", 5), null);
});

test("nextSort cycles ascending, descending, none; a new column starts ascending", () => {
  let s = nextSort(null, "a");
  assert.deepEqual(s, { key: "a", dir: 1 });
  s = nextSort(s, "a");
  assert.deepEqual(s, { key: "a", dir: -1 });
  assert.equal(nextSort(s, "a"), null);
  assert.deepEqual(nextSort({ key: "a", dir: -1 }, "b"), { key: "b", dir: 1 });
});

test("counterText says all rows or a share of them", () => {
  assert.match(counterText(5, 5), /^5 filas$/);
  assert.match(counterText(2, 5000), /^2 de 5\.?000$/);
});
