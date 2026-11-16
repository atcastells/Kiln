// The ONLY place where the walkthrough knows how the page is built. The rework
// of the UI must keep the user-visible parts (button labels, aria-labels) and
// update this map for anything else. Everything here is a CSS selector, a
// visible label, or the source of a small in-page predicate.

export const S = {
  // Top-level views, found as buttons by their visible text.
  views: { context: "Contexto", tree: "Árbol", content: "Contenido", catalog: "Catálogo" },
  reloadLabel: "Releer el proyecto",
  backLabel: "Volver",

  // The search box, by its accessible name.
  search: 'input[aria-label^="Buscar"]',
  // Dropdown with matching assets under the search box, and its entries.
  hits: "#hits",
  hitItem: "#hits button",

  // Graph views: one element per item (node), with its title and kind label.
  item: ".node",
  itemTitle: ".t",
  itemKind: ".hd",
  itemDimmed: "dim", // class that marks an item filtered out by the search
  // List view of a content set: table rows, and the heading of each set.
  content: "#contentView",
  contentRow: "#contentView tr",
  contentHeading: "#contentView h2",
  // Side panel: what fills in when something is selected.
  side: "#side",
  chat: '[aria-label="Hablar con el agente"]',
  projectTag: "#proj",
  wizard: "#wizard",

  // Console noise that is not the page's fault.
  ignoredConsole: [/favicon\.ico/],

  // "First meaningful paint": the page has loaded the project, drawn items
  // and filled the side panel. Source of a function evaluated in the page.
  meaningful: `() => document.querySelector('#proj')?.textContent !== 'cargando…'
    && document.querySelector('.node') && document.querySelector('#side')?.innerText.trim()`,
};
