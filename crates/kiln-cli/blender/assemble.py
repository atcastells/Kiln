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
from mathutils import Euler, Vector


def plan_path():
    if "--" in sys.argv:
        return sys.argv[sys.argv.index("--") + 1]
    if len(sys.argv) > 1 and sys.argv[1].endswith(".json"):
        return sys.argv[1]
    return os.environ["KILN_PLAN"]


def to_blender(v):
    return Vector((v[0], -v[2], v[1]))


ZONES = {}


def dress(o, item):
    """Gives every material slot of the copy its `part/zone` material."""
    if not o.data.materials:
        o.data.materials.append(None)
    for i, slot in enumerate(o.data.materials):
        zone = zone_of(slot)
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


def import_model(path):
    """The glTF as one mesh object, with its base centred on the origin."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
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
    return sum(len(p.vertices) - 2 for p in o.data.polygons)


def collection(name):
    c = bpy.data.collections.get(name)
    if c is None:
        c = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(c)
    return c


def build(item, cache):
    model = item.get("model")
    o = None
    if model and os.path.exists(model):
        if model not in cache:
            src = import_model(model)
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
    if item["fit"] or model is None:
        o.scale = Vector(
            tuple(t / s if s > 1e-6 else 1.0 for t, s in zip(target, dims))
        )
    rx, ry, rz = (math.radians(a) for a in item["turn"])
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
    o["kiln_part"] = item["part"]
    o["kiln_kind"] = item["kind"]
    if item.get("moves"):
        o["kiln_moves"] = item["moves"]
    if item.get("only_in"):
        o["kiln_only_in"] = ",".join(item["only_in"])
    if item.get("todo"):
        o["kiln_todo"] = item["todo"]
    return o


def main():
    plan = json.load(open(plan_path(), encoding="utf-8"))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    cache, built = {}, []
    for item in plan["items"]:
        built.append((item, build(item, cache)))
    for src, _ in cache.values():
        mesh = src.data
        bpy.data.objects.remove(src)
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)
    os.makedirs(os.path.dirname(plan["glb"]), exist_ok=True)
    os.makedirs(os.path.dirname(plan["blend"]), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=plan["blend"])
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=plan["glb"], export_format="GLB", export_extras=True
    )
    corners = []
    for _, o in built:
        corners += [o.matrix_world @ Vector(c) for c in o.bound_box]
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
