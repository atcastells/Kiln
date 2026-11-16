// Drives the page the way a user does (real mouse and key events through CDP)
// and waits for the DOM to settle. Knows nothing of the page except `S`.
import { Page, sleep } from "./cdp.mjs";
import { S } from "./selectors.mjs";

const q = JSON.stringify;

const INSTRUMENT = `(() => {
  const k = window.__k = { lastMut: 0, lastNet: 0, t0: 0, fmp: null, fmpPaint: null, pending: 0, long: 0 };
  const meaningful = ${S.meaningful};
  const of = window.fetch.bind(window);
  window.fetch = (...a) => { k.pending++; return of(...a).finally(() => { k.pending--; k.lastNet = performance.now(); }); };
  new MutationObserver(() => {
    k.lastMut = performance.now();
    if (k.fmp == null) { try { if (meaningful()) { k.fmp = k.lastMut; requestAnimationFrame(() => requestAnimationFrame(() => { k.fmpPaint = performance.now(); })); } } catch {} }
  }).observe(document, { subtree: true, childList: true, attributes: true, characterData: true });
  try { new PerformanceObserver(l => l.getEntries().forEach(e => { k.long += e.duration; })).observe({ type: "longtask" }); } catch {}
  const mark = () => { k.t0 = performance.now(); };
  addEventListener("click", mark, true); addEventListener("keydown", mark, true);
  window.__vis = el => !!el && el.getClientRects().length > 0 && getComputedStyle(el).visibility !== "hidden";
})()`;

export const find = {
  button: label => `() => [...document.querySelectorAll('button')].find(b => __vis(b) && b.textContent.trim() === ${q(label)})`,
  item: title => `() => [...document.querySelectorAll(${q(S.item)})].find(n => __vis(n) && n.querySelector(${q(S.itemTitle)})?.textContent.trim() === ${q(title)})`,
  itemKind: kind => `() => [...document.querySelectorAll(${q(S.item)})].find(n => __vis(n) && (n.querySelector(${q(S.itemKind)})?.textContent.trim() ?? '').startsWith(${q(kind)}))`,
  hit: () => `() => [...document.querySelectorAll(${q(S.hitItem)})].find(__vis)`,
  search: () => `() => document.querySelector(${q(S.search)})`,
};

export class Driver {
  constructor(page, errors) { this.page = page; this.errors = errors; }

  static async open(chromePort, label) {
    const page = await Page.open(chromePort);
    const errors = [];
    const ignored = t => S.ignoredConsole.some(r => r.test(t));
    page.on("Runtime.exceptionThrown", e => errors.push(`[${label}] exception: ${e.exceptionDetails.exception?.description || e.exceptionDetails.text}`));
    page.on("Runtime.consoleAPICalled", e => { if (e.type === "error" || e.type === "assert") { const t = e.args.map(a => a.value ?? a.description ?? "").join(" "); if (!ignored(t)) errors.push(`[${label}] console.${e.type}: ${t}`); } });
    page.on("Log.entryAdded", ({ entry }) => { if (entry.level === "error" && !ignored(entry.text + (entry.url || ""))) errors.push(`[${label}] log: ${entry.text} ${entry.url || ""}`); });
    for (const d of ["Runtime", "Page", "Log", "Network"]) await page.send(d + ".enable");
    await page.send("Network.setCacheDisabled", { cacheDisabled: true });
    await page.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
    await page.send("Page.addScriptToEvaluateOnNewDocument", { source: INSTRUMENT });
    return new Driver(page, errors);
  }

  async goto(url) { await this.page.send("Page.navigate", { url }); }
  arm() { return this.page.eval("(() => { const k = __k; k.lastMut = 0; k.lastNet = 0; k.long = 0; k.t0 = performance.now(); })()"); }

  // Waits until nothing changed in the DOM and no request is pending for `quiet` ms.
  // Returns the time from the user action to the last DOM change, in ms.
  async settle({ quiet = 150, timeout = 20000 } = {}) {
    const start = Date.now();
    for (;;) {
      await sleep(30);
      let s;
      try { s = await this.page.eval("({ p: __k.pending, m: Math.max(__k.lastMut, __k.lastNet), now: performance.now(), t0: __k.t0, long: __k.long })"); }
      catch { if (Date.now() - start > timeout) throw new Error("page never became scriptable"); continue; }
      if (s.p === 0 && s.now - s.m >= quiet) return { ms: Math.max(0, s.m - s.t0), changed: s.m > 0, long: s.long };
      if (Date.now() - start > timeout) throw new Error(`DOM did not settle in ${timeout} ms`);
    }
  }
  async waitFor(fnSrc, timeout = 15000) {
    const start = Date.now();
    while (Date.now() - start < timeout) { if (await this.page.eval(`!!(${fnSrc})()`).catch(() => false)) return true; await sleep(50); }
    return false;
  }

  // Real mouse click on the element a finder returns; falls back to el.click()
  // when the element is outside the visible area (reported as `programmatic`).
  async click(finder, { count = 1 } = {}) {
    const box = await this.page.eval(`(() => { const el = (${finder})(); if (!el) return null;
      const r = el.getBoundingClientRect(), x = r.left + Math.min(r.width / 2, 60), y = r.top + Math.min(r.height / 2, 14), top = document.elementFromPoint(x, y);
      return { x, y, hit: !!top && (top === el || el.contains(top)) }; })()`);
    if (!box) throw new Error("nothing to click: " + finder.slice(0, 120));
    await this.arm();
    if (box.hit) await this.page.clickAt(box.x, box.y, count);
    else await this.page.eval(`(${finder})().click()`);
    return { programmatic: !box.hit };
  }
  async clickAndSettle(finder, opts) { const c = await this.click(finder, opts); const s = await this.settle(); return { ...c, ...s }; }

  async focusSearch() { await this.click(find.search()); }
  async type(text) { for (const ch of text) { await this.page.key(ch); await sleep(12); } }
  async clearSearch() {
    await this.focusSearch();
    await this.page.eval(`(() => { const el = (${find.search()})(); el.setSelectionRange(0, el.value.length); })()`); // select all, as Ctrl+A
    await this.page.key("Backspace");
  }

  items() {
    return this.page.eval(`(() => [...document.querySelectorAll(${q(S.item)})].filter(__vis).map(n => ({
      title: n.querySelector(${q(S.itemTitle)})?.textContent.trim() ?? '', kind: n.querySelector(${q(S.itemKind)})?.textContent.trim() ?? '',
      dim: n.classList.contains(${q(S.itemDimmed)}) })))()`);
  }
  side() { return this.page.eval(`document.querySelector(${q(S.side)})?.innerText.trim() ?? ''`); }
  visible(sel) { return this.page.eval(`[...document.querySelectorAll(${q(sel)})].some(__vis)`); }
  buttonVisible(label) { return this.page.eval(`!!(${find.button(label)})()`); }
  domCount() { return this.page.eval("document.getElementsByTagName('*').length"); }
  async heapMB() { await this.page.send("HeapProfiler.collectGarbage").catch(() => {}); const h = await this.page.send("Runtime.getHeapUsage"); return h.usedSize / 1048576; }
  contentRows() { return this.page.eval(`[...document.querySelectorAll(${q(S.contentRow)})].filter(r => r.querySelector('td')).length`); }
  contentHeadings() { return this.page.eval(`[...document.querySelectorAll(${q(S.contentHeading)})].map(h => h.textContent.trim())`); }
  hits() { return this.page.eval(`__vis(document.querySelector(${q(S.hits)})) ? [...document.querySelectorAll(${q(S.hitItem)})].map(b => b.innerText.trim()) : []`); }
  close() { return this.page.close(); }
}
