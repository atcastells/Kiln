# Kiln: measures the pieces only Godot can read and exports a GLB of each.
# Run by `kiln godot`:
#   godot --headless --path <godot project> --script measure.gd -- requests.json out.json
# requests.json: [{ "key", "res", "glb" }]; out.json: { key: { size_m, min, tris, material_slots, glb } }.
# Sizes are in metres with Y up, as Godot and glTF use them.
extends SceneTree


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var requests = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	var out := {}
	for r in requests:
		out[r["key"]] = _measure(r)
	var f := FileAccess.open(args[1], FileAccess.WRITE)
	f.store_string(JSON.stringify(out, "  "))
	f.close()
	quit()


func _measure(r: Dictionary) -> Dictionary:
	var res = load(r["res"])
	if res == null:
		return {"error": "Godot no la carga"}
	var root: Node3D
	if res is Mesh:
		root = Node3D.new()
		var mi := MeshInstance3D.new()
		mi.mesh = res
		root.add_child(mi)
		mi.owner = root
	elif res is PackedScene:
		var n = res.instantiate()
		if not n is Node3D:
			n.free()
			return {"error": "la escena no es 3D"}
		root = n
	else:
		return {"error": "es un %s, no una malla ni una escena" % res.get_class()}
	root.name = String(r["key"]).get_file()

	var box := AABB()
	var first := true
	var tris := 0
	var slots: Array[String] = []
	var instances: Array = root.find_children("*", "MeshInstance3D", true, false)
	if root is MeshInstance3D:
		instances.append(root)
	for mi in instances:
		var mesh: Mesh = mi.mesh
		if mesh == null:
			continue
		var b: AABB = _to_root(mi, root) * mesh.get_aabb()
		box = b if first else box.merge(b)
		first = false
		for s in mesh.get_surface_count():
			var arrays := mesh.surface_get_arrays(s)
			var idx = arrays[Mesh.ARRAY_INDEX]
			var n: int = idx.size() if idx != null and idx.size() > 0 else arrays[Mesh.ARRAY_VERTEX].size()
			tris += n / 3
			var name := _slot_name(mi.get_active_material(s), s)
			if not slots.has(name):
				slots.append(name)

	DirAccess.make_dir_recursive_absolute(String(r["glb"]).get_base_dir())
	var doc := GLTFDocument.new()
	var state := GLTFState.new()
	var err := doc.append_from_scene(root, state)
	if err == OK:
		err = doc.write_to_filesystem(state, r["glb"])
	root.free()
	var result := {
		"size_m": [box.size.x, box.size.y, box.size.z],
		"min": [box.position.x, box.position.y, box.position.z],
		"tris": tris,
		"material_slots": slots,
	}
	if err == OK:
		result["glb"] = r["glb"]
	return result


func _to_root(n: Node3D, root: Node) -> Transform3D:
	var t := Transform3D.IDENTITY
	var cur: Node = n
	while cur != null and cur != root:
		if cur is Node3D:
			t = cur.transform * t
		cur = cur.get_parent()
	return t


func _slot_name(m: Material, s: int) -> String:
	if m == null:
		return "surface_%d" % s
	if m.resource_name != "":
		return m.resource_name
	if m.resource_path != "" and not m.resource_path.contains("::"):
		return m.resource_path.get_file().get_basename()
	return "surface_%d" % s
