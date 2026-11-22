"""Kiln: builds an asset in Blender from its assembly plan (plan.json).

Run by `kiln build`, either inside Blender:
    blender --background --factory-startup --python assemble.py -- plan.json
or with a Python that has the `bpy` module:
    python assemble.py plan.json
It can also be pasted into a live Blender (MCP) with KILN_PLAN set.

The plan is in glTF axes (Y up, metres); Blender is Z up, so a point
(x, y, z) of the plan is (x, -z, y) here, and the glTF exporter turns it back.
Writes the .blend, the .glb, report.json and materials.json beside them.
Kiln puts paint.py and split.py in front of this file.
"""

import json
import math
import os
import sys
from datetime import datetime, timezone

import bpy
from mathutils import Euler, Matrix, Vector


def plan_path():
    if "--" in sys.argv:
        return sys.argv[sys.argv.index("--") + 1]
    if len(sys.argv) > 1 and sys.argv[1].endswith(".json"):
        return sys.argv[1]
    return os.environ["KILN_PLAN"]


def to_blender(v):
    return Vector((v[0], -v[2], v[1]))


ZONES = {}
# models that came with a skeleton: placed once, never copied
RIGS = set()


def dress(o, item):
    """Gives every material slot of the copy its `part/zone` material."""
    if not o.data.materials:
        o.data.materials.append(None)
    nested = item["kind"] == "asset"
    for i, slot in enumerate(o.data.materials):
        # a built asset is a closed box: its zones keep their whole name
        # inside it (`post/Wood`) and wear what they wore there
        zone = (slot.get("kiln_name") if nested and slot is not None else None) or zone_of(slot)
        name = item["part"] + "/" + zone
        if name not in ZONES:
            paint = (
                item.get("zones", {}).get(zone)
                or item.get("material")
                or item.get("slots", {}).get(zone)
            )
            if slot is not None:
                keep = slot.copy()
                keep.name = name + OWN
                keep["kiln_zone"] = zone
                keep.use_fake_user = True
            ZONES[name] = wear(name, paint, slot)
        o.data.materials[i] = ZONES[name]


def meshes_of(o):
    """The meshes of a piece: itself, or those under the root of a rigged one."""
    if not o.get("kiln_rig"):
        return [o]
    return ([o] if o.type == "MESH" else []) + [c for c in o.children_recursive if c.type == "MESH"]


def import_model(path):
    """The glTF as one mesh object, with its base centred on the origin.

    A model with a skeleton stays as it came instead: its root is returned,
    with hierarchy, weights and actions untouched.
    """
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path, disable_bone_shape=True)
    new = [o for o in bpy.data.objects if o not in before]
    if any(o.type == "ARMATURE" for o in new):
        roots = [o for o in new if o.parent is None]
        root = roots[0]
        if len(roots) > 1:
            root = bpy.data.objects.new("rig_root", None)
            bpy.context.scene.collection.objects.link(root)
            for o in roots:
                o.parent = root
        root["kiln_rig"] = True
        return root
    meshes = [o for o in new if o.type == "MESH"]
    for o in new:
        if o.type != "MESH":
            continue
        # bake the hierarchy into the mesh before dropping the empties
        o.data = o.data.copy()
        o.data.transform(o.matrix_world)
        o.parent = None
        o.matrix_world.identity()
    for o in new:
        if o.type != "MESH":
            bpy.data.objects.remove(o)
    if not meshes:
        return None
    bpy.ops.object.select_all(action="DESELECT")
    for o in meshes:
        o.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    return bpy.context.view_layer.objects.active


def box():
    bpy.ops.mesh.primitive_cube_add(size=1.0)
    o = bpy.context.view_layer.objects.active
    return o


def tube_rims(t):
    """The two rims of a tube, in glTF space: the rule of `kiln_core::model::tube_rims`.

    Plain Python floats (f64), same operations in the same order as the Rust
    side: mathutils.Vector is float32 and breaks near-ties of the reference
    axis differently, which turns the rim and changes the bounds.
    """

    def cross(a, b):
        return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]

    def unit(a):
        n = math.sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2])
        return [a[0] / n, a[1] / n, a[2] / n]

    d = unit([t["to"][i] - t["from"][i] for i in range(3)])
    k = min(range(3), key=lambda i: abs(d[i]))
    a = [1.0 if k == i else 0.0 for i in range(3)]
    e1 = unit(cross(d, a))
    e2 = cross(d, e1)
    n = t["segments"]

    def rim(c, r):
        out = []
        for i in range(n):
            th = math.tau * i / n
            s, co = math.sin(th), math.cos(th)
            out.append([c[j] + r * (co * e1[j] + s * e2[j]) for j in range(3)])
        return out

    r0 = t["radius"]
    return [rim(t["from"], r0), rim(t["to"], t.get("radius_to") or r0)]


def shape_faces(bm, shape, slot):
    """Adds one shape to the bmesh, in Blender axes, all its faces on `slot`."""
    faces = []
    if "tube" in shape:
        r0, r1 = ([bm.verts.new(to_blender(p)) for p in rim] for rim in tube_rims(shape["tube"]))
        n = len(r0)
        for i in range(n):
            j = (i + 1) % n
            faces.append(bm.faces.new((r0[i], r0[j], r1[j], r1[i])))
        faces.append(bm.faces.new(r0))
        faces.append(bm.faces.new(r1))
    else:
        if "box" in shape:
            lo, hi = shape["box"]
            o = [min(lo[i], hi[i]) for i in range(3)]
            u, v, w = ([hi[i] - lo[i] if i == k else 0.0 for i in range(3)] for k in range(3))
            u, v, w = ([abs(c) for c in e] for e in (u, v, w))
        else:
            p = shape["para"]
            o, u, v, w = p["origin"], p["u"], p["v"], p["w"]
        o, u, v, w = Vector(o), Vector(u), Vector(v), Vector(w)
        c = [bm.verts.new(to_blender(o + a * u + b * v + g * w)) for a in (0, 1) for b in (0, 1) for g in (0, 1)]
        # corner index is 4a + 2b + g
        for q in ((0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)):
            faces.append(bm.faces.new([c[i] for i in q]))
    for f in faces:
        f.material_index = slot


def modelled(item, uv_per_m):
    """The object of a `model` part: its shapes in one mesh, one material slot per zone."""
    import bmesh

    zones = []
    for s in item["shapes"]:
        if s["zone"] not in zones:
            zones.append(s["zone"])
    bm = bmesh.new()
    for s in item["shapes"]:
        shape_faces(bm, s, zones.index(s["zone"]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    uv = bm.loops.layers.uv.new("UVMap")
    for f in bm.faces:
        n = f.normal
        axis = max(range(3), key=lambda i: abs(n[i]))
        i, j = [k for k in range(3) if k != axis]
        for lp in f.loops:
            lp[uv].uv = (lp.vert.co[i] * uv_per_m, lp.vert.co[j] * uv_per_m)
    me = bpy.data.meshes.new(item["name"])
    bm.to_mesh(me)
    bm.free()
    for z in zones:
        m = bpy.data.materials.new(z)
        # the zone travels in a property: Blender may cut or suffix the name
        m["kiln_zone"] = z
        me.materials.append(m)
    return bpy.data.objects.new(item["name"], me)


def base_to_origin(o):
    """Moves the mesh so the centre of its base is at the origin; returns its size (Blender axes)."""
    vs = [v.co for v in o.data.vertices]
    lo = Vector((min(v.x for v in vs), min(v.y for v in vs), min(v.z for v in vs)))
    hi = Vector((max(v.x for v in vs), max(v.y for v in vs), max(v.z for v in vs)))
    shift = Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z))
    for v in o.data.vertices:
        v.co -= shift
    return hi - lo


def tris(o):
    return sum(len(p.vertices) - 2 for m in meshes_of(o) for p in m.data.polygons)


def collection(name):
    c = bpy.data.collections.get(name)
    if c is None:
        c = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(c)
    return c


def tag(o, item):
    """What the spec says of the part, as custom properties the GLB carries as extras."""
    o["kiln_part"] = item["part"]
    o["kiln_kind"] = item["kind"]
    if item.get("moves"):
        o["kiln_moves"] = item["moves"]
    if item.get("only_in"):
        o["kiln_only_in"] = ",".join(item["only_in"])
    if item.get("todo"):
        o["kiln_todo"] = item["todo"]


def build_rig(item, o):
    """Places a rigged piece once, where and how it was authored: no bake, no fit, no recentring."""
    if item["fit"] or item.get("split"):
        raise SystemExit(
            "assemble.py: una pieza con esqueleto no se ajusta de tamaño ni se divide: " + item["part"]
        )
    if not meshes_of(o):
        raise SystemExit("assemble.py: la pieza con esqueleto no tiene ninguna malla: " + item["part"])
    # the root takes the name of the copy; a child that had it keeps a name of its own
    for c in o.children_recursive:
        if c.name == item["name"]:
            c.name = item["name"] + "_mesh"
    o.name = item["name"]
    rx, ry, rz = (math.radians(a) for a in item["turn"])
    turn = Euler((rx, -rz, ry), "XYZ").to_matrix().to_4x4()
    o.matrix_world = Matrix.Translation(to_blender(item["at"])) @ turn @ o.matrix_world
    parts = collection("parts")
    for x in [o] + list(o.children_recursive):
        for c in x.users_collection:
            c.objects.unlink(x)
        parts.objects.link(x)
    bpy.context.view_layer.update()
    for m in meshes_of(o):
        dress(m, item)
    for x in [o] + meshes_of(o):
        tag(x, item)
    return o


def build(item, cache, uv_per_m=1.0):
    model = item.get("model")
    o = None
    if item.get("shapes"):
        o = modelled(item, uv_per_m)
        dims = base_to_origin(o)
    elif model and os.path.exists(model):
        if model in RIGS:
            raise SystemExit(
                "assemble.py: una pieza con esqueleto se coloca una sola vez: " + item["part"]
            )
        if model not in cache:
            src = import_model(model)
            if src is not None and src.get("kiln_rig"):
                RIGS.add(model)
                return build_rig(item, src)
            if src is not None:
                src.name = "src_" + os.path.basename(model)
                if item.get("split"):
                    split(src, item["split"])
                cache[model] = (src, base_to_origin(src))
                for c in src.users_collection:
                    c.objects.unlink(src)
        if model in cache:
            src, dims = cache[model]
            o = src.copy()
            o.data = src.data.copy()
    if o is None:
        o = box()
        for c in o.users_collection:
            c.objects.unlink(o)
        dims = base_to_origin(o)
    o.name = item["name"]
    w, h, d = item["size_m"]
    target = Vector((w, d, h))  # glTF (w, h, d) is Blender (x=w, y=d, z=h)
    if item["fit"] or (model is None and not item.get("shapes")):
        o.scale = Vector(
            tuple(t / s if s > 1e-6 else 1.0 for t, s in zip(target, dims))
        )
    rx, ry, rz = (math.radians(a) for a in item["turn"])
    # an imported glTF comes in quaternion mode, which ignores rotation_euler
    o.rotation_mode = "XYZ"
    o.rotation_euler = Euler((rx, -rz, ry), "XYZ")
    # `at` is the centre of the base of the copy as it stands, once turned
    o.location = Vector()
    collection("todo" if item.get("todo") else "parts").objects.link(o)
    bpy.context.view_layer.update()
    corners = [o.matrix_world @ Vector(c) for c in o.bound_box]
    base = Vector(
        (
            (min(c.x for c in corners) + max(c.x for c in corners)) / 2,
            (min(c.y for c in corners) + max(c.y for c in corners)) / 2,
            min(c.z for c in corners),
        )
    )
    o.location = to_blender(item["at"]) - base
    dress(o, item)
    tag(o, item)
    return o


def anchor(a):
    """An empty where the asset joins something else; glTF axes in, Blender axes out."""
    o = bpy.data.objects.new(a["name"], None)
    o.empty_display_size = 0.2
    x, y, z = a["at"]
    o.location = Vector((x, -z, y))
    rx, ry, rz = (math.radians(d) for d in a["turn"])
    o.rotation_mode = "XYZ"
    o.rotation_euler = Euler((rx, -rz, ry), "XYZ")
    collection("anchors").objects.link(o)
    return o


def main():
    plan = json.load(open(plan_path(), encoding="utf-8"))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    cache, built = {}, []
    for item in plan["items"]:
        built.append((item, build(item, cache, plan.get("uv_per_m") or 1.0)))
    for a in plan.get("anchors", []):
        anchor(a)
    for src, _ in cache.values():
        mesh = src.data
        bpy.data.objects.remove(src)
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)
    # the materials of the models as imported, now replaced by the worn ones
    for me in [me for me in bpy.data.meshes if me.users == 0]:
        bpy.data.meshes.remove(me)
    for m in [m for m in bpy.data.materials if m.users == 0]:
        bpy.data.materials.remove(m)
    os.makedirs(os.path.dirname(plan["glb"]), exist_ok=True)
    os.makedirs(os.path.dirname(plan["blend"]), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=plan["blend"])
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=plan["glb"], export_format="GLB", export_extras=True
    )
    # A rig is measured at rest, like the bounds of its GLB, not in the pose its first action starts in.
    rigs = [a for a in bpy.data.objects if a.type == "ARMATURE"]
    posed = [a.data.pose_position for a in rigs]
    for a in rigs:
        a.data.pose_position = "REST"
    bpy.context.view_layer.update()
    corners = []
    for _, o in built:
        for m in meshes_of(o):
            corners += [m.matrix_world @ Vector(c) for c in m.bound_box]
    for a, was in zip(rigs, posed):
        a.data.pose_position = was
    lo = Vector((min(c.x for c in corners), min(c.y for c in corners), min(c.z for c in corners)))
    hi = Vector((max(c.x for c in corners), max(c.y for c in corners), max(c.z for c in corners)))
    size = hi - lo
    report = {
        "size_m": [round(size.x, 3), round(size.z, 3), round(size.y, 3)],
        "tris": sum(tris(o) for _, o in built),
        "items": [
            {"name": i["name"], "tris": tris(o), "todo": i.get("todo")} for i, o in built
        ],
        "built_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    }
    with open(plan["report"], "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    with open(plan["materials"], "w", encoding="utf-8") as f:
        json.dump(worn(), f, indent=2, ensure_ascii=False, sort_keys=True)
    print("kiln: built", plan["glb"], report["tris"], "tris")


main()
