"""Kiln: splits a model of one material slot into zones.

Kiln puts this file in front of assemble.py. Two ways (`kiln/zones.toml`):
`loose`, a zone per piece of the mesh that touches no other (`part_1`...);
`texture`, a zone per colour of the model's colour map under each face
(`color_1`...). Zones are numbered from the one with most faces.
"""

import bpy

MAX_ZONES = 8
# A colour that covers less than this share of the faces joins its nearest.
MIN_SHARE = 0.04


def islands(mesh):
    """Face index -> the id of the loose piece it belongs to."""
    # glTF repeats a vertex for every face that meets at a hard edge, so
    # vertices in the same place count as one
    place = {}
    at = [
        place.setdefault(tuple(round(c, 4) for c in v.co), len(place))
        for v in mesh.vertices
    ]
    parent = list(range(len(place)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for p in mesh.polygons:
        first = find(at[p.vertices[0]])
        for v in p.vertices[1:]:
            parent[find(at[v])] = first
    return [find(at[p.vertices[0]]) for p in mesh.polygons]


def colour_map(mesh):
    """The image on the base colour of the model's material, if it has one."""
    for m in mesh.materials:
        if m is None or m.node_tree is None:
            continue
        for n in m.node_tree.nodes:
            if n.type == "BSDF_PRINCIPLED" and n.inputs["Base Color"].is_linked:
                src = n.inputs["Base Color"].links[0].from_node
                if src.type == "TEX_IMAGE" and src.image is not None:
                    return src.image
    return None


def colours(mesh):
    """Face index -> its colour in the colour map, coarsened to a few steps."""
    img = colour_map(mesh)
    if img is None or not mesh.uv_layers or not img.size[0]:
        return None
    w, h = img.size
    px = img.pixels[:]
    uv = mesh.uv_layers.active.data
    out = []
    for p in mesh.polygons:
        u = sum(uv[i].uv.x for i in p.loop_indices) / p.loop_total
        v = sum(uv[i].uv.y for i in p.loop_indices) / p.loop_total
        at = (int(v % 1.0 * h) % h * w + int(u % 1.0 * w) % w) * 4
        out.append(tuple(round(c * 3) for c in px[at : at + 3]))
    return out


def merge_small(groups):
    """Folds the rare colours into the nearest common one."""
    count = {}
    for g in groups:
        count[g] = count.get(g, 0) + 1
    ranked = sorted(count, key=lambda g: (-count[g], g))
    keep = [g for g in ranked if count[g] >= MIN_SHARE * len(groups)][:MAX_ZONES] or ranked[:1]
    near = {
        g: min(keep, key=lambda k: sum((a - b) ** 2 for a, b in zip(g, k)))
        for g in ranked
    }
    return [near[g] for g in groups]


def split(o, mode):
    """Gives `o` a material slot per zone. Returns the zone names, or none."""
    mesh = o.data
    if len(mesh.materials) > 1 or not mesh.polygons:
        return None
    if mode == "loose":
        groups, prefix = islands(mesh), "part"
    elif mode == "texture":
        groups, prefix = colours(mesh), "color"
        if groups is None:
            return None
        groups = merge_small(groups)
    else:
        return None
    count = {}
    for g in groups:
        count[g] = count.get(g, 0) + 1
    ranked = sorted(count, key=lambda g: (-count[g], g))
    if len(ranked) < 2:
        return None
    if mode == "loose" and len(ranked) > MAX_ZONES:
        ranked = ranked[:MAX_ZONES]
    own = mesh.materials[0] if mesh.materials else None
    mesh.materials.clear()
    names = []
    for n in range(len(ranked)):
        m = own.copy() if own is not None else bpy.data.materials.new("zone")
        m["kiln_zone"] = f"{prefix}_{n + 1}"
        mesh.materials.append(m)
        names.append(m["kiln_zone"])
    slot = {g: n for n, g in enumerate(ranked)}
    for p, g in zip(mesh.polygons, groups):
        # loose pieces beyond the limit go with the last zone
        p.material_index = slot.get(g, len(ranked) - 1)
    return names
