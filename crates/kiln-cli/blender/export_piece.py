"""Kiln: exports the meshes of a .blend to a GLB, so the catalog can measure the
piece and the assembly can use it.

    blender --background --factory-startup --python export_piece.py -- piece.blend out.glb

Scripts embedded in the .blend never run. Only meshes go out, with their
modifiers applied; cameras, lights and empties stay in the .blend. When the
scene has an armature, it goes out too, with the skins and every action.
"""

import os
import sys

import bpy


def args():
    rest = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else sys.argv[1:]
    if len(rest) != 2:
        raise SystemExit("export_piece.py: se esperan el .blend y el .glb")
    return rest


def main():
    blend, glb = args()
    bpy.ops.wm.open_mainfile(filepath=blend, use_scripts=False)
    meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    if not meshes:
        raise SystemExit("export_piece.py: el .blend no tiene ninguna malla en su escena")
    rigged = any(o.type == "ARMATURE" for o in bpy.context.scene.objects)
    keep = ("MESH", "ARMATURE") if rigged else ("MESH",)
    for o in bpy.context.scene.objects:
        o.hide_set(False)
        o.hide_viewport = False
        o.select_set(o.type in keep)
    os.makedirs(os.path.dirname(glb), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=glb,
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
        export_cameras=False,
        export_lights=False,
        export_animations=rigged,
        export_skins=rigged,
        export_animation_mode="ACTIONS",
    )
    print("kiln: exported", glb, len(meshes), "meshes")


main()
