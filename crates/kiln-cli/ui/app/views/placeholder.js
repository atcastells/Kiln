// Stand-in for a view that is not built yet: says so and points to the old UI.
import { html } from "../html.js";
import { LABELS } from "../nav.js";

export function mount(el, ctx) {
  const name = LABELS[ctx.route.view] ?? ctx.route.view;
  el.innerHTML = html`<div class="page placeholder" data-view="placeholder">
    <div class="pagehead"><div><h1>${name}</h1></div></div>
    <div class="empty">
      <h3>Esta vista llega en el siguiente encargo</h3>
      <p>Mientras tanto, ${name} sigue disponible en la interfaz anterior, con todo lo que ya hacía.</p>
      <a class="btn primary" href="/">Abrir la interfaz anterior</a>
    </div>
  </div>`.toString();
  return () => { el.innerHTML = ""; };
}
