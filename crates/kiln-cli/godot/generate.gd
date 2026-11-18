# Kiln: builds the mesh of a generated asset from resolved recipe operations and exports a GLB.
# Run by `kiln generate`:
#   godot --headless --path <empty project> --script generate.gd -- job.json out.json
# job.json: { "id", "glb", "material": key | null,
#             "ops": [{ "op", "params": { name: number }, "noise_seed"?: int }] }
# out.json: { size_m, min, tris, godot } or { error }; exit code 1 on error.
# Every value arrives resolved: this script draws nothing. Sizes are metres, Y up.
extends SceneTree


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var job = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	var result := _build(job)
	var f := FileAccess.open(args[1], FileAccess.WRITE)
	f.store_string(JSON.stringify(result, "  "))
	f.close()
	quit(1 if result.has("error") else 0)


func _build(job: Dictionary) -> Dictionary:
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

	var root := Node3D.new()
	root.name = String(job["id"])
	var mi := MeshInstance3D.new()
	mi.name = String(job["id"])
	mi.mesh = mesh
	root.add_child(mi)
	mi.owner = root

	var box := mesh.get_aabb()
	DirAccess.make_dir_recursive_absolute(String(job["glb"]).get_base_dir())
	var doc := GLTFDocument.new()
	var state := GLTFState.new()
	var err := doc.append_from_scene(root, state)
	if err == OK:
		err = doc.write_to_filesystem(state, String(job["glb"]))
	root.free()
	if err != OK:
		return {"error": "Godot no exporta el GLB (código %d)" % err}
	return {
		"size_m": [box.size.x, box.size.y, box.size.z],
		"min": [box.position.x, box.position.y, box.position.z],
		"tris": tris,
		"godot": Engine.get_version_info()["string"],
	}


# Unshared vertices with one normal per triangle. SurfaceTool.generate_normals would
# still smooth coincident vertices of smooth group 0, so the face normals are set here.
func _flat(arrays: Array) -> ArrayMesh:
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
