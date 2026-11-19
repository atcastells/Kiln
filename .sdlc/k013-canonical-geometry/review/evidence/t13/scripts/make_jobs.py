import json, os, sys
kiln, recipes, out, glb_root = sys.argv[1:5]
script = open(os.path.join(kiln, "crates/kiln-cli/godot/generate.gd")).read()
builder = open(os.path.join(kiln, "crates/kiln-cli/godot/kiln_generator.gd")).read()
project = 'config_version=5\n\n[application]\n\nconfig/name="kiln-verify"\n'
names = []
for n in (32, 64, 128):
    rec = json.load(open(os.path.join(recipes, f"asteroid_{n}/recipe.json")))
    for s in rec["seeds"]:
        name = f"pub_{n}_{s['seed']}"
        d = os.path.join(out, name)
        os.makedirs(d, exist_ok=True)
        for f, t in (("project.godot", project), ("generate.gd", script), ("kiln_generator.gd", builder)):
            open(os.path.join(d, f), "w").write(t)
        job = {"id": name, "glb": f"{glb_root}/{name}/seed.glb", "material": s["material"], "ops": s["ops"]}
        json.dump(job, open(os.path.join(d, "job.json"), "w"), separators=(",", ":"))
        names.append(f"{name} {s['ops'][0]['params']['segments']}")
open(os.path.join(out, "cases.txt"), "w").write("\n".join(names) + "\n")
print(len(names))
