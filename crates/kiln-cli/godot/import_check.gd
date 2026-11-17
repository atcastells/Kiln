# Kiln: checks that Godot can load and instance an imported GLB.
# Run by `kiln verify`, after `godot --headless --path <project> --import`:
#   godot --headless --path <project> --script import_check.gd -- res://asset.glb out.json
# out.json: { ok, error, nodes: [names], meshes, size_m: [w, h, d] }, metres, Y up.
extends SceneTree


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var out := {"ok": false, "error": "", "nodes": [], "meshes": 0, "size_m": [0.0, 0.0, 0.0]}
	if args.size() != 2:
		push_error("import_check.gd: se esperan la escena y el fichero de salida")
		quit(2)
		return
	var res = load(args[0])
	if res == null or not (res is PackedScene):
		out["error"] = "Godot no carga %s como escena" % args[0]
	else:
		var root: Node = res.instantiate()
		if root == null:
			out["error"] = "Godot no puede instanciar %s" % args[0]
		else:
			var box := {"set": false, "aabb": AABB()}
			_walk(root, Transform3D.IDENTITY, out, box)
			var size: Vector3 = box["aabb"].size
			out["size_m"] = [size.x, size.y, size.z]
			out["ok"] = true
			root.free()
	var f := FileAccess.open(args[1], FileAccess.WRITE)
	f.store_string(JSON.stringify(out, "  "))
	f.close()
	quit(0 if out["ok"] else 1)


func _walk(node: Node, parent: Transform3D, out: Dictionary, box: Dictionary) -> void:
	var xform := parent
	if node is Node3D:
		xform = parent * (node as Node3D).transform
	out["nodes"].append(String(node.name))
	if node is MeshInstance3D and (node as MeshInstance3D).mesh != null:
		out["meshes"] += 1
		var local: AABB = xform * (node as MeshInstance3D).mesh.get_aabb()
		box["aabb"] = local if not box["set"] else (box["aabb"] as AABB).merge(local)
		box["set"] = true
	for child in node.get_children():
		_walk(child, xform, out, box)
