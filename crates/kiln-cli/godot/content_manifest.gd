# Kiln: lists the nodes of the given scenes that carry an id, for `kiln content-manifest`.
#   godot --headless --path <godot project> --script content_manifest.gd -- in.json out.json
# in.json:  { "set": name, "id_meta": key, "scenes": ["res://..."] }
# out.json: { "version": 2, "set", "scenes": [ { "scene", "nodes": [ { "id", "meta", "children": [ { "name", "meta" } ] } ], "error"? } ] }
# Reads the scene state without instancing it, so no node script runs.
# A node takes part when it has the metadata `id_meta`; its direct children are listed with their metadata.
extends SceneTree


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var job = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	var paths: Array = job["scenes"]
	paths.sort()
	var out := []
	var failed := []
	for path in paths:
		var scene := _scene(path, job["id_meta"])
		if scene.has("error"):
			failed.append(path)
		out.append(scene)
	var f := FileAccess.open(args[1], FileAccess.WRITE)
	f.store_string(JSON.stringify({"version": 2, "set": job["set"], "scenes": out}, "  ", true))
	f.close()
	for path in failed:
		printerr("kiln: Godot no carga la escena ", path)
	quit(1 if failed.size() > 0 else 0)


func _json(value):
	match typeof(value):
		TYPE_STRING, TYPE_INT, TYPE_FLOAT, TYPE_BOOL:
			return value
		TYPE_VECTOR2I, TYPE_VECTOR2:
			return [value.x, value.y]
		TYPE_VECTOR3I, TYPE_VECTOR3:
			return [value.x, value.y, value.z]
		TYPE_ARRAY, TYPE_PACKED_STRING_ARRAY, TYPE_PACKED_INT32_ARRAY, TYPE_PACKED_INT64_ARRAY, TYPE_PACKED_FLOAT32_ARRAY, TYPE_PACKED_FLOAT64_ARRAY:
			var items := []
			for v in value:
				items.append(_json(v))
			return items
	return str(value)


func _meta(state: SceneState, i: int) -> Dictionary:
	var meta := {}
	for p in state.get_node_property_count(i):
		var name := state.get_node_property_name(i, p)
		if name.begins_with("metadata/"):
			meta[name.trim_prefix("metadata/")] = _json(state.get_node_property_value(i, p))
	return meta


func _parent(path: String) -> String:
	var cut := path.rfind("/")
	return path.substr(0, cut) if cut >= 0 else "."


func _scene(path: String, id_meta: String) -> Dictionary:
	var packed = load(path) if ResourceLoader.exists(path) else null
	if not packed is PackedScene:
		return {"scene": path, "nodes": [], "error": "Godot no la carga"}
	var state: SceneState = packed.get_state()
	var paths := []
	for i in state.get_node_count():
		paths.append(str(state.get_node_path(i)))
	var nodes := []
	for i in state.get_node_count():
		var meta := _meta(state, i)
		if not meta.has(id_meta):
			continue
		var id = meta[id_meta]
		meta.erase(id_meta)
		var children := []
		for j in state.get_node_count():
			if j != i and _parent(paths[j]) == paths[i]:
				children.append({"name": str(state.get_node_name(j)), "meta": _meta(state, j)})
		nodes.append({"id": str(id), "meta": meta, "children": children})
	return {"scene": path, "nodes": nodes}
