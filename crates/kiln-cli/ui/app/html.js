// `html` tagged template: every interpolation is escaped (text and attributes
// written with double or single quotes) unless it is a `raw()` value or the
// result of another `html` call. Attributes must be quoted.
class Safe {
  constructor(s) { this.s = s; }
  toString() { return this.s; }
}

const MAP = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
export const esc = v => String(v).replace(/[&<>"']/g, c => MAP[c]);
export const raw = s => new Safe(String(s));

function render(v) {
  if (v == null || v === false) return "";
  if (v instanceof Safe) return v.s;
  if (Array.isArray(v)) return v.map(render).join("");
  return esc(v);
}

export function html(strings, ...values) {
  let out = strings[0];
  for (let i = 0; i < values.length; i++) out += render(values[i]) + strings[i + 1];
  return new Safe(out);
}

// The URL only when its scheme is http or https; anything else, `#`.
export function safeUrl(u) {
  if (typeof u !== "string") return "#";
  try {
    const p = new URL(u.trim()).protocol;
    return p === "http:" || p === "https:" ? u.trim() : "#";
  } catch { return "#"; }
}
