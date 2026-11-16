// Route name -> loader of its view module. A view exports `mount(el, ctx)`
// and returns its `unmount`. Add a view: write the module, add a line here.

export const REGISTRY = {
  home: () => import("./home.js"),
  context: () => import("./context.js"),
  tree: () => import("./tree.js"),
  content: () => import("./content.js"),
  catalog: () => import("./catalog.js"),
  materials: () => import("./materials.js"),
  jobs: () => import("./jobs.js"),
  spec: () => import("./spec.js"),
  build: () => import("./build.js"),
  review: () => import("./build.js"),
  job: () => import("./jobs.js"),
  new: () => import("./wizard.js"),
};
