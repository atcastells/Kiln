// The navigation model: what each entry shows (counter, severity dot). Pure,
// computed from the catalog, its derived indexes, the context and the jobs.
import { worstOf } from "./derive.js";
import { fmtN } from "./format.js";

export const LABELS = { home: "Inicio", context: "Contexto", tree: "Árbol", content: "Contenido", catalog: "Catálogo", materials: "Materiales", jobs: "Trabajos" };

export const isRunning = j => j.state === "running";

export function navModel({ catalog: cat, derived: d, context, jobs = [] }) {
  const running = jobs.filter(isRunning).length;
  const open = context?.context?.questions?.length ?? 0;
  const claims = context?.context?.claims?.length ?? 0;
  const elements = (cat.content ?? []).reduce((n, s) => n + s.elements.length, 0);
  const groups = [
    { title: null, items: [{ view: "home", label: LABELS.home, sev: null, count: "" }] },
    { title: "Estructura", items: [
      { view: "context", label: LABELS.context, sev: open ? "info" : null, count: open ? `${fmtN(open)} abiertas` : fmtN(claims) },
      { view: "tree", label: LABELS.tree, sev: worstOf(d.areas.tree), count: fmtN(Object.keys(cat.tree?.nodes ?? {}).length) },
    ] },
    { title: "Datos", items: [
      ...((cat.content ?? []).length ? [{ view: "content", label: LABELS.content, sev: worstOf(d.areas.content), count: fmtN(elements) }] : []),
      { view: "catalog", label: LABELS.catalog, sev: worstOf(d.areas.catalog), count: fmtN(cat.pieces.length) },
      { view: "materials", label: LABELS.materials, sev: worstOf(d.areas.materials), count: fmtN((cat.materials ?? []).length) },
    ] },
    { title: "Actividad", items: [
      { view: "jobs", label: LABELS.jobs, sev: running ? "busy" : null, count: running ? `${fmtN(running)} en curso` : "" },
    ] },
  ];
  return groups;
}
