import test from "node:test";
import assert from "node:assert/strict";
import { html, raw, safeUrl, esc } from "../../app/html.js";

const s = x => String(x);

test("text interpolations are escaped", () => {
  const evil = `<img src=x onerror="alert(1)">&'`;
  assert.equal(s(html`<p>${evil}</p>`), "<p>&lt;img src=x onerror=&quot;alert(1)&quot;&gt;&amp;&#39;</p>");
});

test("double-quoted attributes cannot be broken out of", () => {
  const evil = `" onmouseover="alert(1)`;
  const out = s(html`<a title="${evil}">x</a>`);
  assert.equal(out, `<a title="&quot; onmouseover=&quot;alert(1)">x</a>`);
  assert.ok(!out.includes(`" onmouseover`));
});

test("raw() passes markup through; html results nest without double escaping", () => {
  assert.equal(s(html`<b>${raw("<i>x</i>")}</b>`), "<b><i>x</i></b>");
  const inner = html`<i>${"a<b"}</i>`;
  assert.equal(s(html`<p>${inner}</p>`), "<p><i>a&lt;b</i></p>");
});

test("lists are concatenated and each item escaped", () => {
  const rows = ["a", "<b>", 3].map(v => html`<li>${v}</li>`);
  assert.equal(s(html`<ul>${rows}</ul>`), "<ul><li>a</li><li>&lt;b&gt;</li><li>3</li></ul>");
  assert.equal(s(html`<ul>${["<x>", "y"]}</ul>`), "<ul>&lt;x&gt;y</ul>");
});

test("null, undefined and false render nothing; numbers and zero render", () => {
  assert.equal(s(html`[${null}${undefined}${false}]`), "[]");
  assert.equal(s(html`[${0}${"0"}]`), "[00]");
});

test("esc escapes the five characters", () => {
  assert.equal(esc(`&<>"'`), "&amp;&lt;&gt;&quot;&#39;");
});

test("safeUrl lets only http and https through", () => {
  assert.equal(safeUrl("https://example.com/a?b=1"), "https://example.com/a?b=1");
  assert.equal(safeUrl("http://example.com"), "http://example.com");
  for (const bad of ["javascript:alert(1)", " JaVaScRiPt:alert(1)", "java\tscript:alert(1)", "data:text/html,<script>", "vbscript:x", "//evil.com", "/relative", "", null, undefined, 42, "file:///etc/passwd"]) {
    assert.equal(safeUrl(bad), "#", String(bad));
  }
});
