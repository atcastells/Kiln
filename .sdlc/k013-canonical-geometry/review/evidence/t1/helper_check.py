# Replica de verify_recipe::tests::sphere_glb (crates/kiln-cli/src/verify_recipe.rs:205-231) para medir su giro
import math, struct, sys, runpy, io, contextlib
W = sys.argv[1]; sys.argv = ['x', W]
with contextlib.redirect_stdout(io.StringIO()):
    g = runpy.run_path(W + '/compare.py')
f32 = lambda x: struct.unpack('<f', struct.pack('<f', x))[0]
for S in (8, 64):
    R = S // 2; pos = []
    for r in range(R + 2):
        th = math.pi * r / (R + 1); ring = 0.0 if r in (0, R + 1) else math.sin(th)
        for s in range(S + 1):
            ph = math.tau * s / S
            pos.append((f32(ring * math.cos(ph)), f32(math.cos(th)), f32(ring * math.sin(ph))))
    idx = []
    for r in range(R + 1):
        for s in range(S):
            a = r * (S + 1) + s; b = a + 1; c = a + S + 1; d = c + 1
            idx += [a, b, c, b, d, c]
    print(f'sphere_glb({S}): f32 winding', g['signs'](pos, idx, True), ' canonical V/T/winding', g['canon'](pos, idx))
