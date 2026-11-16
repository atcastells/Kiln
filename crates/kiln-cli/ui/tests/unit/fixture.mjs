// A small catalog shaped like /api/catalog, for the view helpers.
export const piece = (kit, name, extra = {}) => ({ key: `${kit}/${name}`, kit, name, file: `kits/${kit}/${name}.glb`, format: "glb", size_m: [1, 2, 3], min: [0, 0, 0], tris: 100, material_slots: ["Wood"], split: null, images: 0, licenses: ["CC0"], credit: null, manifest: {}, measured_by: "glb", preview: null, bindings: {}, ...extra });

export const catalog = () => ({
  project: "Demo", root: "/x",
  kits: [{ path: "kits/harbor", name: "harbor", licenses: ["CC0"], has_license_file: true, has_credit: false, pieces: 3 }, { path: "kits/town", name: "town", licenses: [], has_license_file: false, has_credit: false, pieces: 1 }],
  pieces: [piece("harbor", "beam", { tris: 12 }), piece("harbor", "Crate_big", { tris: 900, material_slots: ["Wood", "Metal"], bindings: { Wood: "pine" } }), piece("harbor", "rope", { tris: null, size_m: null }), piece("town", "Árbol")],
  materials: [{ key: "pine", name: "Pino", source: "table", file: "data/materials.toml", detail: { color: [1, 0, 0] }, color: [1, 0, 0], albedo: null }, { key: "Metal", name: "Metal", source: "texture", file: "tex/metal.png", detail: {}, color: null, albedo: "tex/metal.png" }],
  specs: [], builds: {},
  tree: { root: "game:game", nodes: {
    "game:game": { key: "game:game", id: "game", level: "game", name: "Juego", file: null, children: ["place:port", "asset:lamp"], status: "group", piece: null, assets: 3, modelled: 1 },
    "place:port": { key: "place:port", id: "port", level: "place", name: "Puerto", file: "specs/port.toml", children: ["asset:crane", "asset:ghost"], status: "group", piece: null, assets: 2, modelled: 1 },
    "asset:crane": { key: "asset:crane", id: "crane", level: "asset", name: "Grúa", file: "specs/crane.toml", children: [], status: "model", piece: "harbor/beam", assets: 1, modelled: 1 },
    "asset:ghost": { key: "asset:ghost", id: "ghost", level: "asset", name: "Fantasma", file: null, children: [], status: "missing", piece: null, assets: 1, modelled: 0 },
    "asset:lamp": { key: "asset:lamp", id: "lamp", level: "asset", name: "Farola", file: "specs/lamp.toml", children: [], status: "spec", piece: null, assets: 1, modelled: 0 },
  } },
  content: [{ name: "world", kinds: [{ name: "Door", count: 1 }, { name: "Sector", count: 2 }], elements: [
    { kind: "Sector", id: "sector:yard", file: "data/world.ron", line: 4 }, { kind: "Sector", id: "sector:hall", file: "data/world.ron", line: 5 }, { kind: "Door", id: "door:a", file: "data/world.ron", line: 6 }], errors: 1, warnings: 0 }],
  findings: [
    { severity: "error", subject: "harbor/Crate_big", code: "tris", message: "demasiados triángulos" },
    { severity: "warn", subject: "harbor/beam", code: "scale", message: "escala rara" },
    { severity: "info", subject: "harbor/beam", code: "note", message: "nota" },
    { severity: "warn", subject: "kits/town", code: "license", message: "sin licencia" },
    { severity: "error", subject: "world:data/world.ron#door:a", code: "door", message: "puerta sin sector" },
    { severity: "warn", subject: "world", code: "set", message: "del conjunto" },
    { severity: "info", subject: "part:crane/arm", code: "needs_approval", message: "espera permiso" },
  ],
});
