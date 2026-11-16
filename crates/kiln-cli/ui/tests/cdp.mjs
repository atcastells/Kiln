// A tiny Chrome DevTools Protocol client: no dependencies (Node >= 22 has
// WebSocket and fetch). Launches headless Chrome and drives one page.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";

export const freePort = () => new Promise((res, rej) => {
  const s = net.createServer();
  s.listen(0, "127.0.0.1", () => { const p = s.address().port; s.close(() => res(p)); });
  s.on("error", rej);
});

export const sleep = ms => new Promise(r => setTimeout(r, ms));

export function findChrome() {
  const c = [process.env.CHROME, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"].filter(Boolean);
  const f = c.find(p => existsSync(p));
  if (!f) throw new Error("no Chrome found; set CHROME=/path/to/chrome");
  return f;
}

export async function launchChrome(userDir) {
  const port = await freePort();
  const proc = spawn(findChrome(), [
    "--headless=new", `--remote-debugging-port=${port}`, `--user-data-dir=${userDir}`,
    "--no-first-run", "--no-default-browser-check", "--disable-extensions", "--disable-background-networking",
    "--window-size=1440,900", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "about:blank",
  ], { stdio: "ignore" });
  const kill = () => { try { proc.kill("SIGKILL"); } catch {} };
  process.on("exit", kill);
  for (let i = 0; i < 100; i++) {
    try { const v = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json(); return { port, proc, kill, version: v.Browser }; }
    catch { await sleep(100); }
  }
  kill(); throw new Error("Chrome did not start");
}

export class Page {
  static async open(port) {
    const t = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
    const ws = new WebSocket(t.webSocketDebuggerUrl);
    await new Promise((res, rej) => { ws.onopen = res; ws.onerror = () => rej(new Error("CDP socket failed")); });
    return new Page(ws, port, t.id);
  }
  constructor(ws, port, id) {
    this.ws = ws; this.port = port; this.id = id; this.n = 0; this.pending = new Map(); this.handlers = {};
    ws.onmessage = ev => {
      const m = JSON.parse(ev.data);
      if (m.id && this.pending.has(m.id)) {
        const { res, rej } = this.pending.get(m.id); this.pending.delete(m.id);
        m.error ? rej(new Error(`${m.error.message}`)) : res(m.result);
      } else if (m.method) (this.handlers[m.method] || []).forEach(h => h(m.params));
    };
  }
  send(method, params = {}) {
    const id = ++this.n;
    return new Promise((res, rej) => { this.pending.set(id, { res, rej }); this.ws.send(JSON.stringify({ id, method, params })); });
  }
  on(method, fn) { (this.handlers[method] ||= []).push(fn); }
  async eval(expr) {
    const r = await this.send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error("eval failed: " + (r.exceptionDetails.exception?.description || r.exceptionDetails.text));
    return r.result.value;
  }
  async close() { try { await fetch(`http://127.0.0.1:${this.port}/json/close/${this.id}`); } catch {} try { this.ws.close(); } catch {} }

  mouse(type, x, y, clickCount = 0) { return this.send("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" ? "none" : "left", clickCount, buttons: type === "mousePressed" ? 1 : 0 }); }
  async clickAt(x, y, count = 1) {
    await this.mouse("mouseMoved", x, y);
    for (let i = 1; i <= count; i++) { await this.mouse("mousePressed", x, y, i); await this.mouse("mouseReleased", x, y, i); }
  }
  async tab(shift = false) {
    const ev = { key: "Tab", code: "Tab", windowsVirtualKeyCode: 9, modifiers: shift ? 8 : 0 };
    await this.send("Input.dispatchKeyEvent", { type: "keyDown", ...ev });
    return this.send("Input.dispatchKeyEvent", { type: "keyUp", ...ev });
  }
  async key(ch) {
    const named = { Enter: 13, Escape: 27, ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39 };
    if (named[ch]) {
      const ev = { key: ch, code: ch, windowsVirtualKeyCode: named[ch] };
      await this.send("Input.dispatchKeyEvent", { type: ch === "Enter" ? "keyDown" : "rawKeyDown", ...ev, ...(ch === "Enter" ? { text: "\r" } : {}) });
      return this.send("Input.dispatchKeyEvent", { type: "keyUp", ...ev });
    }
    if (ch === "Backspace") {
      await this.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8 });
      return this.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8 });
    }
    await this.send("Input.dispatchKeyEvent", { type: "keyDown", key: ch, text: ch, unmodifiedText: ch });
    return this.send("Input.dispatchKeyEvent", { type: "keyUp", key: ch });
  }
  // A shortcut such as Ctrl+K: `modifiers` is the CDP bit mask (1 alt, 2 ctrl, 4 meta, 8 shift).
  async combo(key, vk, modifiers) {
    const ev = { key, code: "Key" + key.toUpperCase(), windowsVirtualKeyCode: vk, modifiers };
    await this.send("Input.dispatchKeyEvent", { type: "rawKeyDown", ...ev });
    return this.send("Input.dispatchKeyEvent", { type: "keyUp", ...ev });
  }
  async screenshot(file, fs) {
    const r = await this.send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(file, Buffer.from(r.data, "base64"));
  }
}
