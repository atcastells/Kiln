// Answers to what the person did: toasts (success/info close by themselves,
// errors stay until closed) and `busy`, which keeps a control from being used
// twice and says why while its work runs.
import { html } from "../html.js";

export const AUTO_DISMISS_MS = 5000;

// `root` holds two live regions: `[data-region=status]` (role=status) for
// success and info, `[data-region=alert]` (role=alert) for errors.
export function createToasts(root, { setTimer = setTimeout, clearTimer = clearTimeout } = {}) {
  const region = k => root.querySelector(`[data-region="${k}"]`);
  function show({ kind = "info", title, text = "", action = null, timeout } = {}) {
    const host = region(kind === "error" ? "alert" : "status");
    const el = root.ownerDocument.createElement("div");
    el.className = `toast ${kind === "error" ? "error" : kind === "ok" ? "ok" : ""}`.trim();
    el.innerHTML = html`<div><strong>${title}</strong>${text ? html`<p>${text}</p>` : ""}${action ? html`<button class="btn sm" type="button" data-toast-action>${action.label}</button>` : ""}</div>
      <button class="iconbtn" type="button" aria-label="Cerrar el aviso" data-toast-close>×</button>`.toString();
    let timer = null;
    const dismiss = () => { if (timer) clearTimer(timer); el.remove(); };
    el.querySelector("[data-toast-close]").addEventListener("click", dismiss);
    el.querySelector("[data-toast-action]")?.addEventListener("click", () => { action.run(); dismiss(); });
    host.appendChild(el);
    const ms = timeout ?? (kind === "error" ? 0 : AUTO_DISMISS_MS);
    if (ms > 0) timer = setTimer(dismiss, ms);
    return dismiss;
  }
  return {
    show,
    ok: (title, text, action) => show({ kind: "ok", title, text, action }),
    info: (title, text, action) => show({ kind: "info", title, text, action }),
    error: (title, text, action) => show({ kind: "error", title, text, action }),
  };
}

// Disables `control` with `reason` as its title while `work` runs, then puts it
// back. Returns what `work` returns; errors pass through after the restore.
export async function busy(control, reason, work) {
  const was = { disabled: control.disabled, title: control.getAttribute("title") };
  control.disabled = true;
  control.setAttribute("title", reason);
  control.setAttribute("aria-busy", "true");
  try { return await work(); }
  finally {
    control.disabled = was.disabled;
    if (was.title == null) control.removeAttribute("title"); else control.setAttribute("title", was.title);
    control.removeAttribute("aria-busy");
  }
}

// Fixed notice shown while the project is not trusted; an older server sends
// no `trusted` field, which counts as trusted, and with no project open there
// is nothing to trust yet.
export function trustNotice(session) {
  if (session?.trusted !== false || !session.project) return "";
  return html`<div class="trust-notice" role="status">Proyecto sin confianza: Kiln no lanzará Blender, Godot ni Claude aquí. Revisa la carpeta y, si te fías, ejecuta en tu terminal <code>kiln trust --project ${session.project}</code> y recarga la página.</div>`.toString();
}
