// Hash routes: `#/view/arg1/arg2?key=value`. parse and format are pure;
// createRouter binds them to a window (injectable for tests).
export const VIEWS = ["home", "context", "tree", "content", "catalog", "spec", "build", "review", "job", "materials", "jobs"];
const HOME = "home";

const dec = s => decodeURIComponent(s.replace(/\+/g, " "));

export function parse(hash, views = VIEWS) {
  const home = { view: HOME, args: [], query: {} };
  try {
    const text = String(hash ?? "").replace(/^#/, "");
    const [path, qs = ""] = text.split(/\?(.*)/s);
    const parts = path.split("/").filter(Boolean).map(dec);
    if (!parts.length || !views.includes(parts[0])) return home;
    const query = {};
    for (const pair of qs.split("&")) {
      if (!pair) continue;
      const i = pair.indexOf("=");
      query[dec(i < 0 ? pair : pair.slice(0, i))] = i < 0 ? "" : dec(pair.slice(i + 1));
    }
    return { view: parts[0], args: parts.slice(1), query };
  } catch { return home; }
}

export function format({ view, args = [], query = {} }) {
  const path = [view, ...args].map(encodeURIComponent).join("/");
  const qs = Object.entries(query)
    .filter(([, v]) => v != null)
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
    .join("&");
  return `#/${path}${qs ? "?" + qs : ""}`;
}

export function createRouter({ win = globalThis.window, views = VIEWS } = {}) {
  const current = () => parse(win.location.hash, views);
  return {
    current,
    navigate: route => { win.location.hash = format(route); },
    onChange(fn, { immediate = false } = {}) {
      const handler = () => fn(current());
      win.addEventListener("hashchange", handler);
      if (immediate) handler();
      return () => win.removeEventListener("hashchange", handler);
    },
  };
}
