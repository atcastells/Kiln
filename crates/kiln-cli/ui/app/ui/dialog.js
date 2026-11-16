// Modal dialogs on the native <dialog>: focus goes in, Esc cancels, Enter
// confirms, and focus returns to what had it. They replace `prompt()`.
import { html } from "../html.js";

let seq = 0;

export function openDialog(doc, markup, { onReady }) {
  return open(doc, markup, { onReady });
}

function open(doc, markup, { onReady }) {
  const opener = doc.activeElement;
  const dlg = doc.createElement("dialog");
  dlg.innerHTML = markup;
  doc.body.appendChild(dlg);
  return new Promise(resolve => {
    let result = null;
    const finish = v => { result = v; dlg.close(); };
    dlg.addEventListener("close", () => {
      dlg.remove();
      if (opener?.isConnected && typeof opener.focus === "function") opener.focus();
      resolve(result);
    });
    dlg.addEventListener("click", e => { if (e.target === dlg) finish(null); });
    onReady(dlg, finish);
    dlg.showModal();
    (dlg.querySelector("[data-focus]") ?? dlg.querySelector("input,button"))?.focus();
  });
}

// Asks for one line of text. Resolves with the trimmed text, or null if the
// person cancels. `validate(text)` returns an error message or "".
export function askText(doc, { title, text = "", label, placeholder = "", initial = "", confirm = "Aceptar", validate = () => "" }) {
  const n = ++seq;
  return open(doc, html`<form class="dlg" method="dialog" novalidate aria-labelledby="dlg-t${n}">
      <h2 id="dlg-t${n}">${title}</h2>${text ? html`<p>${text}</p>` : ""}
      <label for="dlg-i${n}">${label}</label>
      <input id="dlg-i${n}" class="field" type="text" autocomplete="off" data-focus placeholder="${placeholder}" value="${initial}" aria-describedby="dlg-e${n}">
      <div class="err" id="dlg-e${n}" role="alert"></div>
      <div class="btns"><button class="btn" type="button" data-cancel>Cancelar</button><button class="btn primary" type="submit">${confirm}</button></div>
    </form>`.toString(), {
    onReady(dlg, finish) {
      const input = dlg.querySelector("input"), err = dlg.querySelector(".err");
      dlg.querySelector("[data-cancel]").addEventListener("click", () => finish(null));
      input.addEventListener("input", () => { err.textContent = ""; input.removeAttribute("aria-invalid"); });
      dlg.querySelector("form").addEventListener("submit", e => {
        e.preventDefault();
        const value = input.value.trim(), problem = validate(value);
        if (problem) { err.textContent = problem; input.setAttribute("aria-invalid", "true"); input.focus(); return; }
        finish(value);
      });
    },
  });
}

// Yes/no. Resolves true or false.
export async function confirmDialog(doc, { title, text = "", confirm = "Aceptar", cancel = "Cancelar" }) {
  const n = ++seq;
  const r = await open(doc, html`<form class="dlg" method="dialog" aria-labelledby="dlg-t${n}">
      <h2 id="dlg-t${n}">${title}</h2>${text ? html`<p>${text}</p>` : ""}
      <div class="btns"><button class="btn" type="button" data-cancel>${cancel}</button><button class="btn primary" type="submit" data-focus>${confirm}</button></div>
    </form>`.toString(), {
    onReady(dlg, finish) {
      dlg.querySelector("[data-cancel]").addEventListener("click", () => finish(null));
      dlg.querySelector("form").addEventListener("submit", e => { e.preventDefault(); finish(true); });
    },
  });
  return r === true;
}
