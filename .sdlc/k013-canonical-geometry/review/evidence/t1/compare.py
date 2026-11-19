import json, struct, sys, math
from collections import Counter
from fractions import Fraction
W = sys.argv[1]

def glb(path):
    b = open(path, 'rb').read()
    assert struct.unpack_from('<I', b, 4)[0] == 2
    jl = struct.unpack_from('<I', b, 12)[0]
    doc = json.loads(b[20:20+jl])
    at = 20 + jl
    bl = struct.unpack_from('<I', b, at)[0]
    bin_ = b[at+8:at+8+bl]
    def acc(i, comp_size_fmt=None):
        a = doc['accessors'][i]; bv = doc['bufferViews'][a['bufferView']]
        start = bv.get('byteOffset', 0) + a.get('byteOffset', 0)
        n = {'SCALAR': 1, 'VEC3': 3}[a['type']]
        fmt = {5126: 'f', 5125: 'I', 5123: 'H', 5121: 'B'}[a['componentType']]
        sz = struct.calcsize('<' + fmt)
        stride = bv.get('byteStride', n * sz)
        out = []
        for k in range(a['count']):
            v = struct.unpack_from('<' + fmt * n, bin_, start + k * stride)
            out.append(v if n > 1 else v[0])
        return out, a['componentType']
    meshes = doc['meshes']
    prims = [p for m in meshes for p in m['primitives']]
    assert len(prims) == 1, len(prims)
    p = prims[0]
    pos, _ = acc(p['attributes']['POSITION'])
    if 'indices' in p:
        idx, ct = acc(p['indices'])
    else:
        idx, ct = list(range(len(pos))), None
    return doc, pos, idx, ct, p

def raw(prefix):
    vb = open(prefix + '.v', 'rb').read()
    v = [struct.unpack_from('<3f', vb, 12 * k) for k in range(len(vb) // 12)]
    try:
        ib = open(prefix + '.i', 'rb').read()
        i = list(struct.unpack('<%di' % (len(ib) // 4), ib))
    except FileNotFoundError:
        i = list(range(len(v)))
    return v, i

def bits(p): return struct.pack('<3f', *p)

def q(x):  # half away from zero on f64(f32)
    y = float(x) * 1000.0
    return int(math.copysign(math.floor(abs(y) + 0.5), y))

def det(a, b, c):
    return (a[0]*(b[1]*c[2]-b[2]*c[1]) - a[1]*(b[0]*c[2]-b[2]*c[0]) + a[2]*(b[0]*c[1]-b[1]*c[0]))

def signs(pos, idx, exact):
    c = Counter()
    for t in range(len(idx) // 3):
        a, b, cc = (pos[idx[3*t+k]] for k in range(3))
        if exact:
            a, b, cc = ([Fraction(x) for x in v] for v in (a, b, cc))
        d = det(a, b, cc)
        c['+' if d > 0 else '-' if d < 0 else '0'] += 1
    return dict(c)

def tri_keys(pos, idx):
    return [tuple(bits(pos[idx[3*t+k]]) for k in range(3)) for t in range(len(idx) // 3)]

def perm_class(g, r):
    if g == r: return 'same'
    if g == (r[0], r[2], r[1]): return 'swap12'
    if g == (r[2], r[1], r[0]): return 'swap02'
    if g == (r[1], r[0], r[2]): return 'swap01'
    if g in ((r[1], r[2], r[0]), (r[2], r[0], r[1])): return 'rotated'
    return 'other'

def canon(pos, idx):
    qv = [tuple(q(x) for x in p) for p in pos]
    ids, verts, remap = {}, [], []
    for v in qv:
        if v not in ids: ids[v] = len(verts); verts.append(v)
        remap.append(ids[v])
    tris = []
    for t in range(len(idx) // 3):
        a, b, c = (remap[idx[3*t+k]] for k in range(3))
        if a != b and b != c and a != c: tris.append((a, b, c))
    s = Counter()
    for a, b, c in tris:
        d = det(verts[a], verts[b], verts[c]); s['+' if d > 0 else '-' if d < 0 else '0'] += 1
    return len(verts), len(tris), dict(s)

S = {'sph8': 8, 'sph64': 64, 'ast42': 64, 'ast42s': 64}
for k in ['sph8', 'sph64', 'ast42s', 'ast42']:
    doc, gp, gi, ct, prim = glb(f'{W}/out/{k}.glb')
    rp, ri = raw(f'{W}/out/{k}.raw')
    rawmeta = json.load(open(f'{W}/out/{k}.raw.json')); out = json.load(open(f'{W}/out/{k}.out.json'))
    print(f'== {k}')
    print(' nodes:', [{kk: vv for kk, vv in n.items() if kk not in ('extras',)} for n in doc['nodes']])
    print(' scenes:', doc.get('scenes'), 'meshes:', len(doc['meshes']), 'prim keys:', sorted(prim.keys()), 'attrs:', sorted(prim['attributes']), 'index ct:', ct, 'mode:', prim.get('mode'))
    print(' counts glb V/I:', len(gp), len(gi), ' raw V/I:', len(rp), len(ri), ' vbytes==12V:', rawmeta['vbytes'] == 12 * rawmeta['verts'], ' out.tris:', out['tris'], ' glb tris:', len(gi) // 3)
    print(' positions same order bitwise:', len(gp) == len(rp) and all(bits(a) == bits(b) for a, b in zip(gp, rp)))
    print(' positions same multiset bitwise:', Counter(map(bits, gp)) == Counter(map(bits, rp)))
    print(' distinct position sets equal:', set(map(bits, gp)) == set(map(bits, rp)))
    print(' index arrays equal:', gi == ri)
    gk, rk = tri_keys(gp, gi), tri_keys(rp, ri)
    print(' triangles by position: same order count:', sum(a == b for a, b in zip(gk, rk)), '/', len(gk))
    print(' per-triangle permutation glb vs raw:', dict(Counter(perm_class(a, b) for a, b in zip(gk, rk))))
    print(' winding f32 exact glb:', signs(gp, gi, True), ' raw:', signs(rp, ri, True))
    V, T, sg = canon(gp, gi)
    exp = (2 + S[k] * S[k] // 2, S[k] * S[k])
    print(' canonical from glb: V', V, 'T', T, 'expected', exp, 'match', (V, T) == exp, 'int winding', sg)
    print(' canonical from raw: ', canon(rp, ri))
