import test from "node:test";
import assert from "node:assert/strict";
import { busy } from "../../app/ui/feedback.js";

function control() {
  const attrs = new Map([["title", "original"]]);
  return {
    disabled: false,
    getAttribute: k => attrs.get(k) ?? null,
    setAttribute: (k, v) => attrs.set(k, v),
    removeAttribute: k => attrs.delete(k),
    attrs,
  };
}

test("busy disables the control with the reason while the work runs, then restores it", async () => {
  const c = control();
  let during;
  const out = await busy(c, "Releyendo", async () => { during = { disabled: c.disabled, title: c.getAttribute("title"), busy: c.getAttribute("aria-busy") }; return 7; });
  assert.equal(out, 7);
  assert.deepEqual(during, { disabled: true, title: "Releyendo", busy: "true" });
  assert.equal(c.disabled, false);
  assert.equal(c.getAttribute("title"), "original");
  assert.equal(c.getAttribute("aria-busy"), null);
});

test("busy restores the control when the work fails and passes the error on", async () => {
  const c = control();
  await assert.rejects(busy(c, "x", async () => { throw new Error("fallo"); }), /fallo/);
  assert.equal(c.disabled, false);
  assert.equal(c.getAttribute("title"), "original");
});

test("busy removes the title it added when there was none", async () => {
  const c = control(); c.attrs.delete("title");
  await busy(c, "x", async () => {});
  assert.equal(c.getAttribute("title"), null);
});
