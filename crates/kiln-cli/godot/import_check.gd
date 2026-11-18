# Kiln: checks that Godot can load and instance an imported GLB.
# Run by `kiln verify`, after `godot --headless --path <project> --import`:
#   godot --headless --path <project> --script import_check.gd -- res://asset.glb out.json
# out.json: { ok, error, nodes: [names], meshes, size_m: [w, h, d],
#   skeletons: [{bones}], animations: [{name, length, moves}] }, metres, Y up.
# `moves` is true when some bone pose changes across 9 instants of the animation.
extends SceneTree


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var out := {
		"ok": false, "error": "", "nodes": [], "meshes": 0, "size_m": [0.0, 0.0, 0.0],
		"skeletons": [], "animations": [],
	}
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
			# Bone poses are only sampled inside the tree, and _init runs before it processes.
			get_root().add_child(root)
			_rig(root, out)
			get_root().remove_child(root)
			root.free()
	var f := FileAccess.open(args[1], FileAccess.WRITE)
	f.store_string(JSON.stringify(out, "  "))
	f.close()
	quit(0 if out["ok"] else 1)


func _find(node: Node, cls: String, found: Array) -> void:
	if node.is_class(cls):
		found.append(node)
	for child in node.get_children():
		_find(child, cls, found)


func _poses(skeletons: Array) -> Array:
	var poses := []
	for s in skeletons:
		for b in range((s as Skeleton3D).get_bone_count()):
			poses.append((s as Skeleton3D).get_bone_pose(b))
	return poses


func _rig(root: Node, out: Dictionary) -> void:
	var skeletons := []
	var players := []
	_find(root, "Skeleton3D", skeletons)
	_find(root, "AnimationPlayer", players)
	for s in skeletons:
		out["skeletons"].append({"bones": (s as Skeleton3D).get_bone_count()})
	for p in players:
		var player := p as AnimationPlayer
		for anim_name in player.get_animation_list():
			var length := player.get_animation(anim_name).length
			var moves := false
			player.play(anim_name)
			player.seek(0.0, true)
			var first := _poses(skeletons)
			for k in range(1, 9):
				player.seek(length * k / 8.0, true)
				var now := _poses(skeletons)
				for i in range(now.size()):
					if not (now[i] as Transform3D).is_equal_approx(first[i]):
						moves = true
						break
				if moves:
					break
			player.stop()
			out["animations"].append({"name": String(anim_name), "length": length, "moves": moves})


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
