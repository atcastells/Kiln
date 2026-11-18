"""Test fixture: a minimal humanoid saved as a .blend.

    blender --background --factory-startup --python rig_humanoid.py -- out.blend

Six bones, one skinned mesh of 72 triangles (24 of its 48 vertices blend two
bones), feet on the origin, facing -Y, and the actions idle and walk.
"""
import math, sys
import bpy
from mathutils import Vector

out_blend = sys.argv[sys.argv.index("--") + 1]
bpy.ops.wm.read_factory_settings(use_empty=True)

# boxes: (bone, centre, size) in Blender axes (Z up, facing -Y)
BOXES = [
    ("spine", (0, 0, 1.15), (0.36, 0.2, 0.5)),
    ("head", (0, 0, 1.55), (0.22, 0.22, 0.24)),
    ("leg_l", (0.1, 0, 0.45), (0.14, 0.16, 0.9)),
    ("leg_r", (-0.1, 0, 0.45), (0.14, 0.16, 0.9)),
    ("arm_l", (0.26, 0, 1.1), (0.1, 0.12, 0.6)),
    ("arm_r", (-0.26, 0, 1.1), (0.1, 0.12, 0.6)),
]
BONES = {  # name: (head, tail, parent)
    "spine": ((0, 0, 0.9), (0, 0, 1.4), None),
    "head": ((0, 0, 1.4), (0, 0, 1.7), "spine"),
    "leg_l": ((0.1, 0, 0.9), (0.1, 0, 0.0), "spine"),
    "leg_r": ((-0.1, 0, 0.9), (-0.1, 0, 0.0), "spine"),
    "arm_l": ((0.26, 0, 1.4), (0.26, 0, 0.8), "spine"),
    "arm_r": ((-0.26, 0, 1.4), (-0.26, 0, 0.8), "spine"),
}

verts, faces, owner = [], [], []
for bone, c, s in BOXES:
    base = len(verts)
    for dx in (-1, 1):
        for dy in (-1, 1):
            for dz in (-1, 1):
                verts.append((c[0] + dx * s[0] / 2, c[1] + dy * s[1] / 2, c[2] + dz * s[2] / 2))
                owner.append(bone)
    for f in ((0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)):
        faces.append(tuple(base + i for i in f))
me = bpy.data.meshes.new("body")
me.from_pydata(verts, [], faces)
mat = bpy.data.materials.new("Suit")
me.materials.append(mat)
body = bpy.data.objects.new("body", me)
bpy.context.scene.collection.objects.link(body)

arm = bpy.data.armatures.new("rig")
rig = bpy.data.objects.new("rig", arm)
bpy.context.scene.collection.objects.link(rig)
bpy.context.view_layer.objects.active = rig
bpy.ops.object.mode_set(mode="EDIT")
for name, (h, t, p) in BONES.items():
    b = arm.edit_bones.new(name)
    b.head, b.tail = Vector(h), Vector(t)
    if p:
        b.parent = arm.edit_bones[p]
bpy.ops.object.mode_set(mode="OBJECT")

for name in BONES:
    g = body.vertex_groups.new(name=name)
    g.add([i for i, o in enumerate(owner) if o == name], 1.0, "REPLACE")
# blended weights: the lower ring of each limb also follows the spine
sp = body.vertex_groups["spine"]
for i, o in enumerate(owner):
    if o in ("leg_l", "leg_r", "arm_l", "arm_r") and verts[i][2] > 0.7:
        body.vertex_groups[o].add([i], 0.6, "REPLACE")
        sp.add([i], 0.4, "REPLACE")
body.parent = rig
body.modifiers.new("Armature", "ARMATURE").object = rig

rig.animation_data_create()
def action(name, frames):
    a = bpy.data.actions.new(name)
    a.use_fake_user = True
    rig.animation_data.action = a
    for frame, poses in frames:
        for bone in BONES:
            pb = rig.pose.bones[bone]
            pb.rotation_mode = "XYZ"
            pb.rotation_euler = (poses.get(bone, 0.0), 0, 0)
            pb.keyframe_insert("rotation_euler", frame=frame)
    return a
sw = math.radians(30)
action("idle", [(1, {"spine": 0.0}), (30, {"spine": math.radians(2)}), (60, {"spine": 0.0})])
action("walk", [
    (1, {"leg_l": sw, "leg_r": -sw, "arm_l": -sw, "arm_r": sw}),
    (12, {"leg_l": -sw, "leg_r": sw, "arm_l": sw, "arm_r": -sw}),
    (24, {"leg_l": sw, "leg_r": -sw, "arm_l": -sw, "arm_r": sw}),
])
bpy.ops.wm.save_as_mainfile(filepath=out_blend)
