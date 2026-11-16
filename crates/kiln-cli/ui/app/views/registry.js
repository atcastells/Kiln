// Route name -> loader of its view module. A view exports `mount(el, ctx)`
// and returns its `unmount`. Add a view: write the module, add a line here.
const placeholder = () => import("./placeholder.js");

export const REGISTRY = {
  home: () => import("./home.js"),
  context: placeholder,
  tree: () => import("./tree.js"),
  content: () => import("./content.js"),
  catalog: () => import("./catalog.js"),
  materials: () => import("./materials.js"),
  jobs: placeholder,
  spec: placeholder,
  build: placeholder,
  review: placeholder,
  job: placeholder,
};
