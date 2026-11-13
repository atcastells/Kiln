"""Kiln: wears the materials of an assembled asset again, without assembling.

Run by `kiln paint` on the .blend that `kiln build` left, with paint.py in
front of this file:
    blender --background --factory-startup --python repaint.py -- repaint.json

repaint.json names the .blend, the .glb to write, the materials.json to
write and, for every `part/zone` material, its palette paint (null: the
model's own). Nothing moves: only the materials change. With `save` the
.blend keeps the change (the asset itself); without it the .glb is a
colour variant and the .blend stays as built.
"""

import json
import sys

import bpy


def job_path():
    if "--" in sys.argv:
        return sys.argv[sys.argv.index("--") + 1]
    return sys.argv[1]


def main():
    job = json.load(open(job_path(), encoding="utf-8"))
    bpy.ops.wm.open_mainfile(filepath=job["blend"])
    done = {}
    for o in [o for o in bpy.data.objects if o.type == "MESH"]:
        for i, slot in enumerate(o.data.materials):
            name = slot.get("kiln_name") if slot is not None else None
            if name not in job["paints"]:
                continue
            if name not in done:
                done[name] = wear(name, job["paints"][name], bpy.data.materials.get(name + OWN))
            o.data.materials[i] = done[name]
    drop_old()
    if job.get("save"):
        bpy.ops.wm.save_mainfile()
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=job["glb"], export_format="GLB", export_extras=True
    )
    with open(job["materials"], "w", encoding="utf-8") as f:
        json.dump(worn(), f, indent=2, ensure_ascii=False, sort_keys=True)
    print("kiln: painted", job["glb"], len(done), "materials")


main()
