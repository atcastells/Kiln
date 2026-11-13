# Vendor

`three.js` is three.js 0.187.0 (MIT, `LICENSE-three.md`) with its GLTFLoader
and OrbitControls, bundled into one ES module so Kiln works offline:

```bash
npm i three@0.187.0 esbuild@0.25.0
npx esbuild three-entry.js --bundle --format=esm --minify --legal-comments=inline --outfile=three.js
```
