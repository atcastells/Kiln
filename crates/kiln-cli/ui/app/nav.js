// The navigation model: what each entry shows (counter, severity dot). Pure,
// computed from the catalog, its derived indexes, the context and the jobs.
import { worstOf } from "./derive.js";

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
      { view: "context", label: LABELS.context, sev: open ? "info" : null, count: open ? `${open} abiertas` : String(claims) },
      { view: "tree", label: LABELS.tree, sev: worstOf(d.areas.tree), count: String(Object.keys(cat.tree?.nodes ?? {}).length) },
    ] },
    { title: "Datos", items: [
      ...((cat.content ?? []).length ? [{ view: "content", label: LABELS.content, sev: worstOf(d.areas.content), count: String(elements) }] : []),
      { view: "catalog", label: LABELS.catalog, sev: worstOf(d.areas.catalog), count: String(cat.pieces.length) },
      { view: "materials", label: LABELS.materials, sev: worstOf(d.areas.materials), count: String((cat.materials ?? []).length) },
    ] },
    { title: "Actividad", items: [
      { view: "jobs", label: LABELS.jobs, sev: running ? "busy" : null, count: running ? `${running} en curso` : "" },
    ] },
  ];
  return groups;
}
