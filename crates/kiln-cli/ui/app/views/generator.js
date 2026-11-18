// Controls for the recipe of a generated asset: pure functions (no DOM) plus a
// thin panel. The vocabulary comes from the catalog (`generator_ops`).
import { html } from "../html.js";

export const clamp = (c, v) => {
  const one = n => { const x = Math.min(c.max, Math.max(c.min, Number(n))); return c.integer ? Math.min(c.max, Math.max(c.min, Math.round(x))) : x; };
  return Array.isArray(v) ? v.map(one).sort((a, b) => a - b) : one(v);
};

export function controlsOf(recipe, vocabulary) {
  return (recipe.ops ?? []).map((o, index) => {
    const def = vocabulary.find(v => v.id === o.op);
    const controls = (def?.params ?? []).map(p => {
      const value = o[p.id] ?? p.default;
      return { param: p.id, label: p.label, unit: p.unit, min: p.min, max: p.max, integer: !!p.integer, step: p.integer ? 1 : "any", range: Array.isArray(value), value };
    });
    return { index, op: o.op, label: def?.label ?? o.op, known: !!def, controls };
  });
}

export const recipeOf = (recipe, edits) => {
  const out = JSON.parse(JSON.stringify(recipe));
  for (const { index, param, value } of edits) if (out.ops[index]) out.ops[index][param] = value;
  return out;
};

export const setSeed = (recipe, seed) => {
  const n = Math.trunc(Number(seed));
  return { ...JSON.parse(JSON.stringify(recipe)), seed: Number.isFinite(n) ? Math.min(4294967295, Math.max(0, n)) : 0 };
};

const num = (c, id, v, label, hidden = false) => html`<input class="field" type="number" id="${id}" data-field="${id}" min="${c.min}" max="${c.max}" step="${c.step}" value="${v}" aria-label="${label}" ${hidden ? "hidden" : ""}>`;

// HTML of the panel; `bindGenerator` makes it live.
export function generatorPanel(spec, vocabulary) {
  const g = spec.generator;
  const groups = controlsOf(g, vocabulary).map(o => html`<fieldset class="card pad" data-op="${o.index}"><legend class="sec">${o.label}</legend>
    ${o.known ? "" : html`<p class="muted small">Operación desconocida: se conserva tal cual.</p>`}
    ${o.controls.map(c => { const id = `g${o.index}_${c.param}`, [lo, hi] = c.range ? c.value : [c.value, c.value];
      return html`<div class="row" data-param="${c.param}"><span>${c.label}${c.unit ? ` (${c.unit})` : ""}</span>
        ${num(c, id + "_lo", lo, c.range ? `${c.label} ${o.label} mín` : `${c.label} ${o.label}`)}${num(c, id + "_hi", hi, `${c.label} ${o.label} máx`, !c.range)}
        <label class="small"><input type="checkbox" data-range="${id}" ${c.range ? "checked" : ""} aria-label="Rango de ${c.label} ${o.label}"> rango</label></div>`; })}</fieldset>`);
  return html`<section class="gen"><h2 class="sec">Receta</h2>${groups}
    <div class="row"><label for="genSeed">Semilla</label><input class="field" id="genSeed" type="number" min="0" max="4294967295" step="1" value="${g.seed}"><button class="btn primary" type="button" data-apply="1">Aplicar</button></div></section>`;
}

export function bindGenerator(root, spec, vocabulary, { onApply }) {
  const q = id => root.querySelector(`#${id}`);
  root.addEventListener("change", e => {
    const id = e.target.dataset?.range; if (!id) return;
    q(id + "_hi").hidden = !e.target.checked;
  });
  root.addEventListener("click", e => {
    if (!e.target.closest?.("[data-apply]")) return;
    const edits = [];
    for (const o of controlsOf(spec.generator, vocabulary)) for (const c of o.controls) {
      const id = `g${o.index}_${c.param}`, lo = Number(q(id + "_lo").value), hi = Number(q(id + "_hi").value);
      const value = clamp(c, root.querySelector(`[data-range="${id}"]`).checked ? [lo, hi] : lo);
      if (JSON.stringify(value) !== JSON.stringify(c.value)) edits.push({ index: o.index, param: c.param, value });
    }
    const seed = Number(root.querySelector("#genSeed").value);
    onApply(setSeed(recipeOf(spec.generator, edits), seed), setSeed(spec.generator, seed).seed);
  });
}
