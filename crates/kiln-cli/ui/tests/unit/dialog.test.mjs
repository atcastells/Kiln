import test from "node:test";
import assert from "node:assert/strict";
import { askText, confirmDialog } from "../../app/ui/dialog.js";
import { openViewerDialog } from "../../app/views/view3d.js";

const HOSTILE = `"><img src=x onerror=alert(1)><script>boom()</script>'`;

// A page that records the markup assigned to the dialog; nothing is parsed.
function fake() {
  const made = [];
  const doc = {
    activeElement: null,
    body: { appendChild() {} },
    createElement() {
      const dlg = { innerHTML: "", addEventListener() {}, remove() {}, close() {}, showModal() {}, querySelector: () => ({ addEventListener() {}, focus() {} }) };
      made.push(dlg);
      return dlg;
    },
  };
  return { doc, made };
}

const noRawTag = markup => assert.ok(!/<(img|script)\b/i.test(markup), markup);

test("askText renders a hostile title, text, label and initial value as text", () => {
  const { doc, made } = fake();
  askText(doc, { title: HOSTILE, text: HOSTILE, label: HOSTILE, placeholder: HOSTILE, initial: HOSTILE, confirm: HOSTILE });
  const m = made[0].innerHTML;
  noRawTag(m);
  assert.ok(m.includes("&lt;script&gt;boom()&lt;/script&gt;"));
  assert.ok(m.includes('value="&quot;&gt;&lt;img src=x onerror=alert(1)&gt;'));
});

test("confirmDialog renders a hostile title, text and button labels as text", () => {
  const { doc, made } = fake();
  confirmDialog(doc, { title: HOSTILE, text: HOSTILE, confirm: HOSTILE, cancel: HOSTILE });
  noRawTag(made[0].innerHTML);
  assert.ok(made[0].innerHTML.includes("&lt;img src=x onerror=alert(1)&gt;"));
});

test("the 3D viewer dialog renders a hostile title as text, in the label too", () => {
  const { doc, made } = fake();
  openViewerDialog(doc, { title: HOSTILE, path: "x.glb" });
  noRawTag(made[0].innerHTML);
  assert.ok(made[0].innerHTML.includes('aria-label="Vista 3D de &quot;&gt;&lt;img'));
});
