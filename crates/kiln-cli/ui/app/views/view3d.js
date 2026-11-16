// The 3D viewer. three.js (`/vendor/three.js`) is loaded the first time one is
// opened; each viewer owns one renderer and frees it, and its WebGL context,
// when it is disposed.
import { html } from "../html.js";
import { openDialog } from "../ui/dialog.js";

export const stats = { created: 0, disposed: 0 };
let three = null;
const loadThree = () => (three ??= import("/vendor/three.js").catch(e => { three = null; throw e; }));
const fileUrl = p => "/files/" + encodeURI(p) + "?t=" + Date.now();

// Puts a viewer of the model at `path` inside `box`. Resolves to { wear, dispose }.
export async function createViewer(box, path, { materials = [] } = {}) {
  let disposed = false, frame = 0, renderer = null, scene = null, ctl = null;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    cancelAnimationFrame(frame);
    ctl?.dispose?.();
    scene?.traverse(o => {
      o.geometry?.dispose?.();
      for (const m of [].concat(o.material ?? [])) { for (const v of Object.values(m)) v?.isTexture && v.dispose(); m.dispose?.(); }
    });
    if (renderer) { renderer.dispose(); renderer.forceContextLoss(); renderer.domElement.remove(); stats.disposed++; }
    renderer = scene = ctl = null;
  };
  const handle = { wear() {}, dispose };
  box.innerHTML = `<span class="note">cargando el modelo…</span>`;
  try {
    const T = await loadThree();
    if (disposed) return handle;
    const w = box.clientWidth || 480, h = box.clientHeight || 320;
    renderer = new T.WebGLRenderer({ antialias: true, alpha: true });
    stats.created++;
    renderer.setPixelRatio(globalThis.devicePixelRatio || 1); renderer.setSize(w, h);
    scene = new T.Scene();
    const cam = new T.PerspectiveCamera(40, w / h, 0.01, 500);
    scene.add(new T.HemisphereLight(0xffffff, 0x666655, 2.2));
    const sun = new T.DirectionalLight(0xffffff, 1.8); sun.position.set(3, 6, 4); scene.add(sun);
    const gltf = await new T.GLTFLoader().loadAsync(fileUrl(path));
    if (disposed) return handle;
    box.innerHTML = ""; box.appendChild(renderer.domElement);
    scene.add(gltf.scene);
    const bb = new T.Box3().setFromObject(gltf.scene), size = bb.getSize(new T.Vector3()), mid = bb.getCenter(new T.Vector3()), r = Math.max(size.x, size.y, size.z) || 1;
    const grid = new T.GridHelper(Math.ceil(r * 2), Math.ceil(r * 2), 0x888888, 0xcccccc); grid.position.y = bb.min.y; scene.add(grid);
    cam.position.set(mid.x + r * 1.2, mid.y + r * 0.8, mid.z + r * 1.6);
    ctl = new T.OrbitControls(cam, renderer.domElement); ctl.target.copy(mid); ctl.update();
    box.insertAdjacentHTML("beforeend", html`<span class="note">${size.x.toFixed(2)} × ${size.y.toFixed(2)} × ${size.z.toFixed(2)} m · arrastra para girar</span>`.toString());
    const tick = () => { if (disposed) return; renderer.render(scene, cam); frame = requestAnimationFrame(tick); };
    tick();
    const first = new Map();
    handle.wear = (name, key) => {
      const m = materials.find(x => x.key === key);
      gltf.scene.traverse(o => { for (const mt of [].concat(o.material ?? [])) {
        if (mt.name !== name) continue;
        if (!first.has(mt)) first.set(mt, { color: mt.color.clone(), map: mt.map });
        const was = first.get(mt);
        if (!m) { mt.color.copy(was.color); mt.map = was.map; }
        else if (m.albedo) { const t = new T.TextureLoader().load("/files/" + encodeURI(m.albedo)); t.colorSpace = T.SRGBColorSpace; t.flipY = false; t.wrapS = t.wrapT = T.RepeatWrapping; mt.map = t; mt.color.setRGB(1, 1, 1); }
        else { mt.map = null; mt.color.setRGB(...(m.color || [0.8, 0.8, 0.8])); }
        mt.needsUpdate = true;
      } });
    };
  } catch (e) {
    if (!disposed) box.innerHTML = html`<span class="note">No se pudo abrir el modelo: ${e.message || e}. Comprueba que el fichero existe (Releer el proyecto) y vuelve a abrirlo.</span>`.toString();
  }
  return handle;
}

// The viewer in a modal dialog; resolves when it is closed (Esc, Cerrar or a click outside).
export function openViewerDialog(doc, { title, path, materials = [] }) {
  let viewer = null, handle = null;
  const done = openDialog(doc, html`<form class="dlg dlg3d" method="dialog" aria-label="Vista 3D de ${title}"><h2>${title}</h2><div class="view3d" data-v3d></div><div class="btns"><button class="btn primary" type="submit" data-focus>Cerrar</button></div></form>`.toString(), {
    onReady(dlg, finish) {
      dlg.querySelector("form").addEventListener("submit", e => { e.preventDefault(); finish(null); });
      dlg.addEventListener("close", () => (handle ? handle.dispose() : viewer?.then(v => v.dispose())), { once: true });
      viewer = createViewer(dlg.querySelector("[data-v3d]"), path, { materials }).then(v => (handle = v));
    },
  });
  // The renderer is freed before the caller's continuation runs.
  return done.then(r => { handle?.dispose(); return r; });
}
