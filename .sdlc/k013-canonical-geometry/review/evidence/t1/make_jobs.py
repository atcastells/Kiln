# usage: python3 make_jobs.py SCRATCH_DIR ASTRA_DIR  -> jobs/{sph8,sph64,ast42,ast42s}.json
import json, sys
W, A = sys.argv[1:]
r = json.load(open(f'{A}/godot/assets/kiln/asteroid_64/recipe.json'))
s = [s for s in r['seeds'] if s['seed'] == 42][0]
def sph(n): return [{"op": "sphere", "params": {"radius_m": 1000.0, "segments": float(n)}}]
jobs = {"sph8": sph(8), "sph64": sph(64), "ast42": s['ops'], "ast42s": [o for o in s['ops'] if o['op'] != 'facet']}
for k, ops in jobs.items():
    json.dump({"id": k, "glb": f"{W}/out/{k}.glb", "material": s['material'] if k.startswith('ast') else None, "ops": ops}, open(f'{W}/jobs/{k}.json', 'w'))
