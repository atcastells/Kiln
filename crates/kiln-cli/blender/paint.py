"""Kiln: the materials of an assembled asset, one per zone of each part.

Kiln puts this file in front of assemble.py and repaint.py. A zone is a
material slot of a part's model; in the asset its material is named
`part/zone`, so it can be worn again later without assembling.
"""

import os
import re

import bpy

OWN = "~own"


def zone_of(slot):
    """The zone a material slot stands for: the name the model gave it."""
    if slot is None:
        return "body"
    return slot.get("kiln_zone") or re.sub(r"\.\d{3}$", "", slot.name) or "body"


def wear(name, paint, own):
    """The material `name`: the palette paint, else the model's own, else a plain one."""
    old = bpy.data.materials.get(name)
    if old is not None:
        old.name = name + "~old"
    if paint is None and own is not None:
        m = own.copy()
        m.use_fake_user = False
    else:
        m = bpy.data.materials.new(name)
    m.name = name
    if paint is not None:
        if m.node_tree is None:  # Blender 4: nodes are off by default
            m.use_nodes = True
        nodes, links = m.node_tree.nodes, m.node_tree.links
        bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
        albedo = paint.get("albedo")
        if albedo and os.path.exists(albedo):
            tex = nodes.new("ShaderNodeTexImage")
            tex.image = bpy.data.images.load(albedo, check_existing=True)
            links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
        elif paint.get("color"):
            bsdf.inputs["Base Color"].default_value = (*paint["color"], 1.0)
    m["kiln_name"] = name
    m["kiln_zone"] = name.split("/", 1)[1]
    m["kiln_palette"] = paint["key"] if paint else ""
    return m


def drop_old():
    for m in [m for m in bpy.data.materials if m.name.endswith("~old")]:
        bpy.data.materials.remove(m)


def worn():
    """`part/zone` -> palette key (none when it wears the model's own)."""
    return {
        m["kiln_name"]: m.get("kiln_palette") or None
        for m in bpy.data.materials
        if m.get("kiln_name") and m.users and not m.name.endswith(OWN)
    }
