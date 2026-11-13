"""Kiln: the review sheets of an assembled asset, and what Blender measures.

Run by `kiln review` after `kiln build`, inside Blender:
    blender --background --factory-startup --python render.py -- asset.blend OUT_DIR
or with a Python that has the `bpy` module:
    python render.py asset.blend OUT_DIR

Writes to OUT_DIR, always with the same cameras, light and background so
two rounds can be compared:
    1-turn.png        six oblique views, 30 degrees up, every 60 degrees
    2-elevations.png  front and side, orthographic, 1 m grid, a 1.8 m figure
    3-parts.png       each part a flat colour with its id, and a legend
    4-silhouette.png  the silhouette, and the asset at its size in the game
    measures.json     boxes, parts that go through others, parts with no support

Measures are in the axes of the plan (glTF: Y up, metres).
"""

import json
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

ENGINE = os.environ.get("KILN_RENDER_ENGINE", "CYCLES")
SAMPLES = int(os.environ.get("KILN_RENDER_SAMPLES", "12"))
GREY = (0.18, 0.18, 0.18)
VIEW = (800, 533)  # one view of the turn sheet


def args():
    a = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else sys.argv[1:]
    return a[0], a[1]


# ---- scene ----


def parts():
    return [o for o in bpy.data.objects if o.type == "MESH" and "kiln_part" in o]


def bounds(objs):
    cs = [o.matrix_world @ Vector(c) for o in objs for c in o.bound_box]
    lo = Vector((min(c.x for c in cs), min(c.y for c in cs), min(c.z for c in cs)))
    hi = Vector((max(c.x for c in cs), max(c.y for c in cs), max(c.z for c in cs)))
    return lo, hi


def emission(name, rgb, strength=1.0):
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    if m.node_tree is None:
        m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    em = nt.nodes.new("ShaderNodeEmission")
    em.inputs["Color"].default_value = (*rgb, 1.0)
    em.inputs["Strength"].default_value = strength
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    nt.links.new(em.outputs["Emission"], out.inputs["Surface"])
    return m


def setup(scene):
    scene.render.engine = ENGINE
    if ENGINE == "CYCLES":
        scene.cycles.samples = SAMPLES
        scene.cycles.use_denoising = False
    scene.render.film_transparent = False
    scene.render.image_settings.file_format = "PNG"
    scene.view_settings.view_transform = "Standard"
    world = bpy.data.worlds.new("kiln_world")
    world.use_nodes = True
    bg = world.node_tree.nodes["Background"]
    bg.inputs["Color"].default_value = (*GREY, 1.0)
    bg.inputs["Strength"].default_value = 1.0
    scene.world = world
    sun = bpy.data.lights.new("kiln_sun", "SUN")
    sun.energy = 3.0
    so = bpy.data.objects.new("kiln_sun", sun)
    so.rotation_euler = (math.radians(50), 0, math.radians(35))
    scene.collection.objects.link(so)
    return bg, so


def camera(scene, name):
    cam = bpy.data.cameras.new(name)
    o = bpy.data.objects.new(name, cam)
    scene.collection.objects.link(o)
    scene.camera = o
    return o


def look(o, target, az, el, dist):
    d = Vector(
        (
            math.cos(math.radians(el)) * math.sin(math.radians(az)),
            -math.cos(math.radians(el)) * math.cos(math.radians(az)),
            math.sin(math.radians(el)),
        )
    )
    o.location = target + d * dist
    o.rotation_euler = (target - o.location).to_track_quat("-Z", "Y").to_euler()


def label(scene, cam, text, res, rgb=(1, 1, 1), at=(0.03, 0.04), size=0.045, name="kiln_label"):
    """Text fixed to the camera, `at` from the top left in fractions of the frame width."""
    scene.render.resolution_x, scene.render.resolution_y = res
    curve = bpy.data.curves.new(name, "FONT")
    curve.body = text
    curve.size = 1
    o = bpy.data.objects.new(name, curve)
    o.data.materials.append(emission(name + "_m", rgb))
    scene.collection.objects.link(o)
    o.parent = cam
    rx, ry = res
    depth = 1.0
    span = cam.data.ortho_scale if cam.data.type == "ORTHO" else 2 * depth * math.tan(cam.data.angle / 2)
    # the span covers the height with a vertical fit, else the longer side
    vertical = cam.data.sensor_fit == "VERTICAL" or (cam.data.sensor_fit == "AUTO" and ry > rx)
    w, h = (span * rx / ry, span) if vertical else (span, span * ry / rx)
    o.location = (-w / 2 + at[0] * w, h / 2 - at[1] * w - size * w, -depth)
    o.scale = (size * w,) * 3
    return o


def render(scene, path, res):
    scene.render.resolution_x, scene.render.resolution_y = res
    scene.render.resolution_percentage = 100
    scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(path, check_existing=False)
    px = np.array(img.pixels[:], dtype=np.float32).reshape(res[1], res[0], 4)
    bpy.data.images.remove(img)
    return px[::-1]  # top row first


def save(px, path):
    h, w = px.shape[:2]
    img = bpy.data.images.new("kiln_sheet", w, h, alpha=True)
    img.pixels = px[::-1].ravel()
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


def grid(tiles, cols):
    rows = [np.concatenate(tiles[i : i + cols], axis=1) for i in range(0, len(tiles), cols)]
    return np.concatenate(rows, axis=0)


def clear(names):
    for o in [o for o in bpy.data.objects if o.name.startswith(names)]:
        bpy.data.objects.remove(o)


# ---- measures ----


def measure(objs):
    """Boxes in glTF axes, parts that go through each other, parts with no support."""

    def gl(v):
        return [round(v.x, 3), round(v.z, 3), round(-v.y, 3)]

    deps = bpy.context.evaluated_depsgraph_get()
    boxes, trees = {}, {}
    for o in objs:
        lo, hi = bounds([o])
        boxes[o.name] = (lo, hi)
        mesh = o.evaluated_get(deps).to_mesh()
        verts = [o.matrix_world @ v.co for v in mesh.vertices]
        polys = [list(p.vertices) for p in mesh.polygons]
        trees[o.name] = BVHTree.FromPolygons(verts, polys)
        o.evaluated_get(deps).to_mesh_clear()
    items = []
    lo_all, hi_all = bounds(objs)
    for o in objs:
        lo, hi = boxes[o.name]
        items.append(
            {
                "name": o.name,
                "part": o["kiln_part"],
                # min and max in glTF axes: x, y up, z towards the viewer
                "min": [round(lo.x, 3), round(lo.z, 3), round(-hi.y, 3)],
                "max": [round(hi.x, 3), round(hi.z, 3), round(-lo.y, 3)],
                "size_m": [round(hi.x - lo.x, 3), round(hi.z - lo.z, 3), round(hi.y - lo.y, 3)],
            }
        )
    through, touching = [], {o.name: set() for o in objs}
    names = [o.name for o in objs]
    for i, a in enumerate(names):
        for b in names[i + 1 :]:
            (la, ha), (lb, hb) = boxes[a], boxes[b]
            gap = max(
                max(la[k] - hb[k], lb[k] - ha[k]) for k in range(3)
            )  # < 0: boxes overlap by that much along the thinnest axis
            if gap <= 0.02:
                touching[a].add(b)
                touching[b].add(a)
            if gap < -0.01 and trees[a].overlap(trees[b]):
                depth = min(min(ha[k], hb[k]) - max(la[k], lb[k]) for k in range(3))
                through.append({"a": a, "b": b, "depth_m": round(depth, 3)})
    on_ground = {n for n in names if boxes[n][0].z <= 0.02}
    # supported: on the ground, or touching something that is (in chain)
    held, grow = set(on_ground), True
    while grow:
        grow = False
        for n in names:
            if n not in held and touching[n] & held:
                held.add(n)
                grow = True
    floating = [
        {
            "name": n,
            "gap_m": round(
                min(
                    [boxes[n][0].z]
                    + [
                        max(max(boxes[n][0][k] - boxes[m][1][k], boxes[m][0][k] - boxes[n][1][k]) for k in range(3))
                        for m in names
                        if m != n
                    ]
                ),
                3,
            ),
        }
        for n in names
        if n not in held
    ]
    return {
        "size_m": [round(hi_all.x - lo_all.x, 3), round(hi_all.z - lo_all.z, 3), round(hi_all.y - lo_all.y, 3)],
        "base_y": round(lo_all.z, 3),
        "items": items,
        "through": through,
        "floating": floating,
        "note": "min/max en ejes glTF (y arriba); through: piezas que se atraviesan más de 1 cm; floating: piezas sin apoyo ni contacto (cadena hasta el suelo)",
    }


# ---- figure ----


def figure(scene, x, y):
    """A 1.8 m person, as plain shapes: legs, body, head."""
    m = emission("kiln_fig_m", (0.85, 0.35, 0.1))
    out = []
    for name, loc, size in [
        ("leg_l", (x - 0.1, y, 0.45), (0.12, 0.14, 0.9)),
        ("leg_r", (x + 0.1, y, 0.45), (0.12, 0.14, 0.9)),
        ("body", (x, y, 1.2), (0.44, 0.24, 0.62)),
        ("arm_l", (x - 0.28, y, 1.15), (0.1, 0.12, 0.64)),
        ("arm_r", (x + 0.28, y, 1.15), (0.1, 0.12, 0.64)),
    ]:
        bpy.ops.mesh.primitive_cube_add(size=1, location=loc)
        o = bpy.context.view_layer.objects.active
        o.scale = size
        o.name = "kiln_fig_" + name
        out.append(o)
    bpy.ops.mesh.primitive_uv_sphere_add(radius=0.14, location=(x, y, 1.66))
    head = bpy.context.view_layer.objects.active
    head.name = "kiln_fig_head"
    out.append(head)
    for o in out:
        o.data.materials.clear()
        o.data.materials.append(m)
    return out


def ground_grid(scene, lo, hi):
    """1 m lines on the ground under the asset and the figure."""
    m = emission("kiln_grid_m", (0.32, 0.32, 0.32))
    x0, x1 = math.floor(lo.x) - 1, math.ceil(hi.x) + 2
    y0, y1 = math.floor(lo.y) - 1, math.ceil(hi.y) + 1
    out = []
    for x in range(x0, x1 + 1):
        bpy.ops.mesh.primitive_cube_add(size=1, location=(x, (y0 + y1) / 2, -0.005))
        o = bpy.context.view_layer.objects.active
        o.scale = (0.012, y1 - y0, 0.01)
        out.append(o)
    for y in range(y0, y1 + 1):
        bpy.ops.mesh.primitive_cube_add(size=1, location=((x0 + x1) / 2, y, -0.005))
        o = bpy.context.view_layer.objects.active
        o.scale = (x1 - x0, 0.012, 0.01)
        out.append(o)
    for o in out:
        o.name = "kiln_grid"
        o.data.materials.append(m)
    return out


# ---- sheets ----

PALETTE = [
    (0.90, 0.30, 0.20), (0.20, 0.55, 0.90), (0.95, 0.75, 0.10), (0.30, 0.75, 0.35),
    (0.65, 0.35, 0.85), (0.10, 0.80, 0.80), (0.95, 0.50, 0.70), (0.55, 0.40, 0.20),
    (0.60, 0.85, 0.20), (0.95, 0.60, 0.20), (0.40, 0.40, 0.95), (0.85, 0.85, 0.85),
]


def main():
    blend, out = args()
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=blend)
    scene = bpy.context.scene
    objs = parts()
    if not objs:
        raise SystemExit("kiln: no parts in " + blend)
    measures = measure(objs)
    with open(os.path.join(out, "measures.json"), "w", encoding="utf-8") as f:
        json.dump(measures, f, indent=2, ensure_ascii=False)

    bg, sun = setup(scene)
    lo, hi = bounds(objs)
    mid = (lo + hi) / 2
    radius = max((hi - lo).length / 2, 0.25)

    # 1 · turn: six oblique views
    cam = camera(scene, "kiln_cam")
    cam.data.lens = 50
    cam.data.sensor_fit = "VERTICAL"
    # the vertical field is the narrow one: the whole asset fits in it
    dist = radius / math.sin(cam.data.angle / 2) * 1.08
    cam.data.clip_end = dist * 4
    tiles = []
    for az in range(0, 360, 60):
        look(cam, mid, az, 30, dist)
        clear("kiln_label")
        label(scene, cam, f"{az}°", VIEW)
        tiles.append(render(scene, os.path.join(out, f"_turn{az}.png"), VIEW))
    clear("kiln_label")
    save(grid(tiles, 3), os.path.join(out, "1-turn.png"))

    # 2 · elevations: front and side, orthographic, with grid, figure and sizes
    size = hi - lo
    fig_x = hi.x + 0.6
    extra = figure(scene, fig_x, mid.y) + ground_grid(scene, lo, Vector((fig_x + 0.3, hi.y, hi.z)))
    cam.data.type = "ORTHO"
    elev = []
    for name, az, span_w in [("frente", 0, (fig_x + 0.4) - lo.x), ("lado", 90, size.y + 1.2)]:
        top = max(hi.z, 1.9)
        span = max(span_w, top) * 1.15
        cam.data.ortho_scale = span
        centre = Vector(((lo.x + fig_x + 0.4) / 2 if az == 0 else mid.x, mid.y, top / 2))
        look(cam, centre, az, 0, dist * 2)
        cam.data.clip_end = dist * 6
        clear("kiln_label")
        label(scene, cam, f"{name} · ancho {size.x:.2f} m · alto {size.z:.2f} m · fondo {size.y:.2f} m · figura 1,80 m · rejilla 1 m", (1000, 1000), size=0.03)
        elev.append(render(scene, os.path.join(out, f"_elev{az}.png"), (1000, 1000)))
    clear("kiln_label")
    for o in extra:
        bpy.data.objects.remove(o)
    cam.data.type = "PERSP"
    save(grid(elev, 2), os.path.join(out, "2-elevations.png"))

    # 3 · parts: flat colours, ids above each part, a legend
    keep = {o.name: list(o.data.materials) for o in objs}
    ids = sorted({o["kiln_part"] for o in objs})
    colour = {pid: PALETTE[i % len(PALETTE)] for i, pid in enumerate(ids)}
    for o in objs:
        m = emission("kiln_part_" + o["kiln_part"], colour[o["kiln_part"]], 0.9)
        o.data.materials.clear()
        o.data.materials.append(m)
    look(cam, mid, 30, 30, dist)
    for i, pid in enumerate(ids):
        label(scene, cam, pid, (1200, 900), colour[pid], at=(0.03, 0.03 + i * 0.04), size=0.032, name="kiln_label_" + pid)
    render_parts = render(scene, os.path.join(out, "_parts.png"), (1200, 900))
    clear("kiln_label")
    save(render_parts, os.path.join(out, "3-parts.png"))

    # 4 · silhouette, and the asset at its size in the game
    black = emission("kiln_black", (0, 0, 0))
    for o in objs:
        o.data.materials.clear()
        o.data.materials.append(black)
    bg.inputs["Color"].default_value = (1, 1, 1, 1)
    look(cam, mid, 20, 15, dist)
    sil = render(scene, os.path.join(out, "_sil.png"), (768, 768))
    for o in objs:
        o.data.materials.clear()
        for m in keep[o.name]:
            o.data.materials.append(m)
    bg.inputs["Color"].default_value = (*GREY, 1)
    small = render(scene, os.path.join(out, "_small.png"), (96, 96))
    big = np.repeat(np.repeat(small, 8, axis=0), 8, axis=1)
    save(grid([sil, big], 2), os.path.join(out, "4-silhouette.png"))

    for f in os.listdir(out):
        if f.startswith("_"):
            os.remove(os.path.join(out, f))
    print("kiln: rendered", out)


main()
