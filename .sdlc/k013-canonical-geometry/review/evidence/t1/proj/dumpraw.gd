extends SceneTree
# usage: godot --headless --path . --script dumpraw.gd -- job.json OUT_PREFIX
func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var job = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	var gen = load("res://kiln_generator.gd")
	var built: Dictionary = gen.build(job)
	var a: Array = built["mesh"].surface_get_arrays(0)
	var v: PackedVector3Array = a[Mesh.ARRAY_VERTEX]
	var f := FileAccess.open(args[1] + ".v", FileAccess.WRITE)
	f.store_buffer(v.to_byte_array())
	f.close()
	var has_idx: bool = a[Mesh.ARRAY_INDEX] != null and (a[Mesh.ARRAY_INDEX] as PackedInt32Array).size() > 0
	if has_idx:
		var idx: PackedInt32Array = a[Mesh.ARRAY_INDEX]
		f = FileAccess.open(args[1] + ".i", FileAccess.WRITE)
		f.store_buffer(idx.to_byte_array())
		f.close()
	f = FileAccess.open(args[1] + ".json", FileAccess.WRITE)
	f.store_string(JSON.stringify({"verts": v.size(), "vbytes": v.to_byte_array().size(), "has_index": has_idx, "tris": built["tris"]}))
	f.close()
	quit()
