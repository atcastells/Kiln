# usage: python3 extra.py SCRATCH_DIR  -> facet vs smooth after quantize and join
import sys, runpy, io, contextlib
W = sys.argv[1]; sys.argv = ['x', W]
with contextlib.redirect_stdout(io.StringIO()):
    g = runpy.run_path(W + '/compare.py')
glb, raw, bits, q = g['glb'], g['raw'], g['bits'], g['q']
_, fp, fi, _, _ = glb(W + '/out/ast42.glb'); _, sp, si, _, _ = glb(W + '/out/ast42s.glb')
print('ast42 facet glb first indices:', fi[:9])
print('sph8 glb first idx:', glb(W + '/out/sph8.glb')[2][:6], 'raw:', raw(W + '/out/sph8.raw')[1][:6])
print('facet distinct positions == smooth distinct positions:', set(map(bits, fp)) == set(map(bits, sp)))
# vertex numbering: first appearance in POSITION order
def canon_full(pos, idx):
    qv = [tuple(q(x) for x in p) for p in pos]; ids = {}; verts = []; remap = []
    for v in qv:
        if v not in ids: ids[v] = len(verts); verts.append(v)
        remap.append(ids[v])
    tris = [tuple(remap[idx[3*t+k]] for k in range(3)) for t in range(len(idx)//3)]
    tris = [t for t in tris if len(set(t)) == 3]
    return verts, [tuple(verts[i] for i in t) for t in tris]
fv, ft = canon_full(fp, fi); sv, st = canon_full(sp, si)
print('quantized vertex sets equal facet vs smooth:', set(fv) == set(sv), ' same first-seen order:', fv == sv)
print('triangles by quantized positions, same order:', ft == st, ' same as sets with cyclic rotation:', sorted(min(t[i:]+t[:i] for i in range(3)) for t in ft) == sorted(min(t[i:]+t[:i] for i in range(3)) for t in st))
