# Kiln: lists the ASTRA ids of the given scenes for `kiln astra-manifest`.
#   godot --headless --path <godot project> --script astra_manifest.gd -- scenes.json out.json
# scenes.json: ["res://..."]; out.json: { "scenes": [ { "scene", "nodes": [ { "id", "connects"? } ], "error"? } ] }.
# Reads the scene state without instancing it, so no node script runs.
# A node takes part when it has the metadata `astra_id`; doors add `astra_connects` (two ids).
extends SceneTree


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var scenes = JSON.parse_string(FileAccess.get_file_as_string(args[0]))
	var out := []
	for path in scenes:
		out.append(_scene(path))
	var f := FileAccess.open(args[1], FileAccess.WRITE)
	f.store_string(JSON.stringify({"scenes": out}, "  ", false))
	f.close()
	quit()


func _scene(path: String) -> Dictionary:
	var packed = load(path) if ResourceLoader.exists(path) else null
	if not packed is PackedScene:
		return {"scene": path, "nodes": [], "error": "Godot no la carga"}
	var state: SceneState = packed.get_state()
	var nodes := []
	for i in state.get_node_count():
		var node := {}
		for p in state.get_node_property_count(i):
			var name := state.get_node_property_name(i, p)
			var value = state.get_node_property_value(i, p)
			if name == "metadata/astra_id":
				node["id"] = str(value)
			elif name == "metadata/astra_connects":
				var ok := false
				if typeof(value) == TYPE_ARRAY or typeof(value) == TYPE_PACKED_STRING_ARRAY:
					ok = value.size() == 2
				if not ok:
					return {"scene": path, "nodes": [], "error": "astra_connects debe ser una lista de dos ids"}
				node["connects"] = [str(value[0]), str(value[1])]
		if node.has("id"):
			nodes.append(node)
	return {"scene": path, "nodes": nodes}
