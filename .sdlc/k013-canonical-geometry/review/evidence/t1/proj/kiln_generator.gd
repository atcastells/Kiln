# Kiln: the mesh builder Kiln publishes with a recipe.
# A game calls `build(job)` with one of the resolved seeds of recipe.json; this script
# draws nothing itself. Sizes are metres, Y up.
#   var gen = load("res://kiln_generator.gd")
#   var built: Dictionary = gen.build(job)
# job:   { "material": key | null,
#          "ops": [{ "op", "params": { name: number }, "noise_seed"?: int }] }
# built: { "mesh": ArrayMesh, "tris": int } or { "error": String }
extends RefCounted


static func build(job: Dictionary) -> Dictionary:
	var radius := 0.0
	var base := PackedVector3Array()
	var verts := PackedVector3Array()
	var arrays := []
	var facet := false
	for o in job["ops"]:
		var p: Dictionary = o["params"]
		match String(o["op"]):
			"sphere":
				radius = float(p["radius_m"])
				var segments := int(p["segments"])
				var sphere := SphereMesh.new()
				sphere.radius = radius
				sphere.height = 2.0 * radius
				sphere.radial_segments = segments
				sphere.rings = segments / 2
				arrays = sphere.get_mesh_arrays()
				base = arrays[Mesh.ARRAY_VERTEX]
				verts = base.duplicate()
			"noise":
				if verts.is_empty():
					return {"error": "la operación noise necesita una esfera antes"}
				var noise := FastNoiseLite.new()
				noise.noise_type = FastNoiseLite.TYPE_SIMPLEX
				noise.seed = int(o["noise_seed"])
				noise.frequency = 1.0 / (float(p["size"]) * radius)
				var amplitude := float(p["amplitude"]) * radius
				for i in verts.size():
					verts[i] += verts[i].normalized() * amplitude * noise.get_noise_3dv(base[i])
			"stretch":
				var k := Vector3(float(p["x"]), float(p["y"]), float(p["z"]))
				for i in verts.size():
					verts[i] *= k
			"facet":
				facet = true
			_:
				return {"error": "operación desconocida: %s" % o["op"]}
	if verts.is_empty():
		return {"error": "la receta no tiene esfera"}

	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = null
	arrays[Mesh.ARRAY_TANGENT] = null
	var mesh: ArrayMesh
	var tris: int = (arrays[Mesh.ARRAY_INDEX] as PackedInt32Array).size() / 3
	if facet:
		mesh = _flat(arrays)
	else:
		var st := SurfaceTool.new()
		st.create_from_arrays(arrays)
		st.generate_normals()
		mesh = st.commit()

	var mat := StandardMaterial3D.new()
	if job["material"] != null:
		mat.resource_name = String(job["material"])
	mesh.surface_set_material(0, mat)
	return {"mesh": mesh, "tris": tris}


# Unshared vertices with one normal per triangle. SurfaceTool.generate_normals would
# still smooth coincident vertices of smooth group 0, so the face normals are set here.
static func _flat(arrays: Array) -> ArrayMesh:
	var idx: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	var src: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uv_src = arrays[Mesh.ARRAY_TEX_UV]
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	for t in idx.size() / 3:
		var a := src[idx[t * 3]]
		var b := src[idx[t * 3 + 1]]
		var c := src[idx[t * 3 + 2]]
		var n := Plane(a, b, c).normal
		# the triangles at the poles have no area: they take the direction from the centre
		if n.length() < 0.5:
			n = a.normalized()
		for j in 3:
			var i := idx[t * 3 + j]
			verts.append(src[i])
			normals.append(n)
			if uv_src != null:
				uvs.append(uv_src[i])
	var out := []
	out.resize(Mesh.ARRAY_MAX)
	out[Mesh.ARRAY_VERTEX] = verts
	out[Mesh.ARRAY_NORMAL] = normals
	if uv_src != null:
		out[Mesh.ARRAY_TEX_UV] = uvs
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, out)
	return mesh
