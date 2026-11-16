// Colour contrast (WCAG 2.x) of every text/background token pair the UI uses,
// in both themes. Text needs 4.5:1; the large or bold text pairs are listed
// with 3:1. Non-text parts (borders of inputs) need 3:1 against their surface.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const css = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "../../app/styles.css"), "utf8");

function tokens(block) {
  const out = {};
  for (const m of block.matchAll(/--([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\b/g)) out[m[1]] = m[2];
  return out;
}
const lightBlock = css.match(/:root\s*\{([^}]*)\}/)[1];
const darkBlock = css.match(/prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{([^}]*)\}/)[1];
export const THEMES = { light: tokens(lightBlock), dark: { ...tokens(lightBlock), ...tokens(darkBlock) } };

const lin = c => { c /= 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
const lum = hex => { const n = parseInt(hex.slice(1), 16); return 0.2126 * lin(n >> 16) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255); };
export const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };

// [text token, background token, minimum]
const TEXT = [
  ["fg", "bg"], ["fg", "panel"], ["fg", "panel2"], ["fg", "line2"],
  ["muted", "bg"], ["muted", "panel"], ["muted", "panel2"], ["muted", "line2"],
  ["accent-text", "bg"], ["accent-text", "panel"], ["accent-text", "panel2"], ["accent-text", "accent-soft"],
  ["accent-fg", "accent"],
  ["ok", "panel"], ["ok", "bg"], ["ok", "okbg"],
  ["warn", "panel"], ["warn", "bg"], ["warn", "warnbg"],
  ["bad", "panel"], ["bad", "bg"], ["bad", "badbg"],
  ["info", "panel"], ["info", "bg"], ["info", "infobg"],
];
const NON_TEXT = [["line-ui", "panel"], ["line-ui", "bg"], ["accent", "panel"], ["accent", "bg"]];

test("the stylesheet defines the tokens in both themes", () => {
  for (const t of Object.values(THEMES)) for (const [a, b] of [...TEXT, ...NON_TEXT]) assert.ok(t[a] && t[b], `${a} / ${b}`);
});

for (const [theme, t] of Object.entries(THEMES)) {
  test(`${theme}: text pairs reach 4.5:1`, () => {
    const bad = TEXT.map(([a, b]) => [a, b, ratio(t[a], t[b])]).filter(([, , r]) => r < 4.5).map(([a, b, r]) => `${a} on ${b}: ${r.toFixed(2)}`);
    assert.deepEqual(bad, []);
  });
  test(`${theme}: control borders and the accent reach 3:1`, () => {
    const bad = NON_TEXT.map(([a, b]) => [a, b, ratio(t[a], t[b])]).filter(([, , r]) => r < 3).map(([a, b, r]) => `${a} on ${b}: ${r.toFixed(2)}`);
    assert.deepEqual(bad, []);
  });
}
