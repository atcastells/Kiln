import test from "node:test";
import assert from "node:assert/strict";
import { createStore } from "../../app/store.js";

const tick = () => new Promise(r => setTimeout(r, 0));

test("get, set and update", () => {
  const s = createStore({ a: 1 });
  assert.equal(s.get("a"), 1);
  s.set("a", 2);
  assert.equal(s.get("a"), 2);
  s.update("a", v => v + 3);
  assert.equal(s.get("a"), 5);
  assert.equal(s.get("missing"), undefined);
});

test("a subscriber runs only when one of its keys changed", async () => {
  const s = createStore({ query: "", route: "home" });
  let q = 0, r = 0;
  s.subscribe("query", () => q++);
  s.subscribe(["route"], () => r++);
  s.set("query", "a");
  await tick();
  assert.deepEqual([q, r], [1, 0]);
  s.set("route", "tree");
  await tick();
  assert.deepEqual([q, r], [1, 1]);
});

test("changes are compared by identity: same value, no call", async () => {
  const s = createStore({ list: [1] });
  const same = s.get("list");
  let n = 0;
  s.subscribe("list", () => n++);
  s.set("list", same);
  await tick();
  assert.equal(n, 0);
  s.set("list", [1]);
  await tick();
  assert.equal(n, 1);
});

test("notifications are batched in one microtask", async () => {
  const s = createStore({ a: 0, b: 0 });
  const calls = [];
  s.subscribe(["a", "b"], keys => calls.push([...keys]));
  s.set("a", 1); s.set("b", 1); s.set("a", 2);
  assert.equal(calls.length, 0, "not synchronous");
  await tick();
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].sort(), ["a", "b"]);
});

test("a key set and put back inside one batch does not notify", async () => {
  const s = createStore({ a: "x" });
  let n = 0;
  s.subscribe("a", () => n++);
  s.set("a", "y"); s.set("a", "x");
  await tick();
  assert.equal(n, 0);
});

test("unsubscribe stops notifications, even for a pending batch", async () => {
  const s = createStore({ a: 0 });
  let n = 0;
  const off = s.subscribe("a", () => n++);
  s.set("a", 1);
  off();
  await tick();
  assert.equal(n, 0);
  s.set("a", 2);
  await tick();
  assert.equal(n, 0);
});

test("a subscriber that writes gets its change delivered in a later batch", async () => {
  const s = createStore({ a: 0, b: 0 });
  const seen = [];
  s.subscribe("a", () => s.set("b", s.get("a") * 2));
  s.subscribe("b", () => seen.push(s.get("b")));
  s.set("a", 2);
  await tick();
  assert.deepEqual(seen, [4]);
});
