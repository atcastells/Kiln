# Test fixture: compares how two imported GLBs of the same rigged model deform.
#   godot --headless --path <project> --script rig_deform.gd -- res://a.glb res://b.glb out.json
# Every vertex is skinned by hand (bones, weights, bind poses) at 9 instants of
# each animation, so bone axes and vertex order do not matter.
# out.json: { verts, samples, missing, worst_m, motion_m }
extends SceneTree


func _up(n: Node) -> Transform3D:
	var t := Transform3D.IDENTITY
	while n != null and n is Node3D:
		t = (n as Node3D).transform * t
		n = n.get_parent()
	return t


func _global_pose(sk: Skeleton3D, b: int) -> Transform3D:
	var t := Transform3D.IDENTITY
	while b >= 0:
		t = sk.get_bone_pose(b) * t
		b = sk.get_bone_parent(b)
	return t


# "animation/instant/rest position" -> skinned position, in the space of the scene root.
func _sample(path: String) -> Dictionary:
	var root: Node = (load(path) as PackedScene).instantiate()
	get_root().add_child(root)
	var player: AnimationPlayer = root.find_children("*", "AnimationPlayer", true, false)[0]
	var sk: Skeleton3D = root.find_children("*", "Skeleton3D", true, false)[0]
	var mi: MeshInstance3D = root.find_children("*", "MeshInstance3D", true, false)[0]
	var arrays := mi.mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var bones: PackedInt32Array = arrays[Mesh.ARRAY_BONES]
	var weights: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
	var per := bones.size() / verts.size()
	var place := _up(sk)
	var out := {}
	for anim in player.get_animation_list():
		player.play(anim)
		for k in 9:
			player.seek(player.get_animation(anim).length * k / 8.0, true)
			var mats: Array[Transform3D] = []
			for i in mi.skin.get_bind_count():
				var name := mi.skin.get_bind_name(i)
				var b := sk.find_bone(name) if name != "" else mi.skin.get_bind_bone(i)
				mats.append(place * _global_pose(sk, b) * mi.skin.get_bind_pose(i))
			for v in verts.size():
				var p := Vector3.ZERO
				for j in per:
					p += weights[v * per + j] * (mats[bones[v * per + j]] * verts[v])
				var rest := (_up(mi) * verts[v]).snappedf(0.001)
				out["%s/%d/%s" % [anim, k, rest]] = [p, _up(mi) * verts[v]]
	get_root().remove_child(root)
	root.free()
	return {"samples": out, "verts": verts.size()}


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	var a := _sample(args[0])
	var b := _sample(args[1])
	var res := {"verts": [a["verts"], b["verts"]], "samples": a["samples"].size(), "missing": 0, "worst_m": 0.0, "motion_m": 0.0}
	for key in a["samples"]:
		if not b["samples"].has(key):
			res["missing"] += 1
			continue
		var pa: Vector3 = a["samples"][key][0]
		res["worst_m"] = maxf(res["worst_m"], pa.distance_to(b["samples"][key][0]))
		res["motion_m"] = maxf(res["motion_m"], pa.distance_to(a["samples"][key][1]))
	res["missing"] += absi(a["samples"].size() - b["samples"].size())
	var f := FileAccess.open(args[2], FileAccess.WRITE)
	f.store_string(JSON.stringify(res))
	f.close()
	quit(0)
