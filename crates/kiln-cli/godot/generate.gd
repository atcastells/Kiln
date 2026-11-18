# Kiln: runner of a generated asset. The mesh is built by kiln_generator.gd (same folder);
# this script only wraps it in a node and exports a GLB.
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
	var gen = load("res://kiln_generator.gd")
	if gen == null:
		return {"error": "no se encuentra kiln_generator.gd"}
	var built: Dictionary = gen.build(job)
	if built.has("error"):
		return built
	var mesh: ArrayMesh = built["mesh"]
	var tris: int = built["tris"]

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
