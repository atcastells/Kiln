// Route name -> loader of its view module. A view exports `mount(el, ctx)`
// and returns its `unmount`. Add a view: write the module, add a line here.
const placeholder = () => import("./placeholder.js");

export const REGISTRY = {
  home: () => import("./home.js"),
  context: placeholder,
  tree: placeholder,
  content: placeholder,
  catalog: placeholder,
  materials: placeholder,
  jobs: placeholder,
  spec: placeholder,
  build: placeholder,
  review: placeholder,
  job: placeholder,
};
