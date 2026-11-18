#!/usr/bin/env bash
# Regression walkthrough and measurements of the web UI in headless Chrome.
#   scripts/ui_test.sh [--update-baseline]
# Needs node >= 22 (to run the tests only) and Chrome (set CHROME to override).
set -euo pipefail
cd "$(dirname "$0")/.."

UPDATE=""
case "${1:-}" in "") ;; --update-baseline) UPDATE="--update-baseline" ;; *) echo "uso: $0 [--update-baseline]" >&2; exit 2 ;; esac
command -v node >/dev/null || { echo "node is required to run the UI tests" >&2; exit 2; }

echo "== unit tests (node --test)"
node --test 'crates/kiln-cli/ui/tests/unit/*.test.mjs' 2>&1 | grep -E '^(not ok|# (tests|pass|fail))' || true
node --test 'crates/kiln-cli/ui/tests/unit/*.test.mjs' >/dev/null 2>&1 || { echo "FAIL: ui unit tests failed" >&2; exit 1; }

OUT="$(mktemp -d "${TMPDIR:-/tmp}/kiln-ui-test.XXXXXX")"
PIDS=()
cleanup() {
  for p in ${PIDS[@]+"${PIDS[@]}"}; do kill "$p" 2>/dev/null || true; done
  pkill -f "user-data-dir=$OUT/chrome-profile" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

cargo build -q
KILN="$PWD/target/debug/kiln"
scripts/ui_fixture.sh "$OUT/synthetic" >/dev/null
mkdir "$OUT/empty"   # a folder without kiln.toml opens the start wizard

# a scratch copy of the examples where a long unbroken string (200 and 5,000
# characters) sits in every kind of field that reaches the catalog view: kit
# folder and piece file (200: the file-name limit), kit manifest fields, licence
# text, a material name bound to a slot, and findings (a ghost piece named by
# the manifest, a spec part that points at a piece that does not exist)
cp -R examples "$OUT/longnames"
node - "$OUT/longnames" <<'JS'
const fs = require("fs"), path = require("path");
const root = process.argv[2], kits = path.join(root, "godot/kits");
const L2 = "k".repeat(200), L5 = "m".repeat(5000), P2 = "p".repeat(200), G2 = "g".repeat(200);
fs.renameSync(path.join(kits, "props"), path.join(kits, L2));
fs.renameSync(path.join(kits, "harbor/crate.glb"), path.join(kits, `harbor/${P2}.glb`));
fs.renameSync(path.join(kits, `${L2}/rope_coil.glb`), path.join(kits, `${L2}/${P2}.glb`));
for (const k of [L2, "harbor"]) {
  fs.writeFileSync(path.join(kits, k, "LICENSE.md"), `# ${L2}\nCC0 1.0\n${L5}\n`);
  fs.writeFileSync(path.join(kits, k, "manifest.json"), JSON.stringify({
    [P2]: { credit: L5, author: L2, source_url: L5, license: "CC0", [L2]: L5 },
    [G2]: { credit: L2, url: L5 },
  }));
}
fs.appendFileSync(path.join(root, "data/materials.toml"),
  `\n[[materials]]\nid = "${L2}"\nname = "${L5}"\ncolor = [0.1, 0.2, 0.3]\n\n[[materials]]\nid = "short_key"\nname = "${L2}"\ncolor = [0.3, 0.2, 0.1]\n`);
fs.mkdirSync(path.join(root, "kiln"), { recursive: true });
fs.writeFileSync(path.join(root, "kiln/bindings.toml"),
  `["harbor/${P2}"]\nWood = "${L2}"\nIron = "short_key"\n\n["${L2}/${P2}"]\n${JSON.stringify(L2)} = "${L2}"\n`);
fs.writeFileSync(path.join(root, "specs/longnames.toml"), `[asset]
id = "longnames"
name = "${L5}"
asked = "${L2}"
status = "layout"
notes = "${L5}"

[[parts]]
id = "${L2}"
what = "${L5}"
count = 1
size_m = [1, 1, 1]
material = "${L2}"
source = { kind = "reuse", from = "${L2}/${G2}", why = "${L5}" }
at = [[0, 0, 0]]
`);
JS

free_port() { node -e 'const s=require("net").createServer().listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})'; }
ARGS=()
serve() { # name dir
  local port; port="$(free_port)"
  "$KILN" serve --project "$2" --port "$port" >"$OUT/serve-$1.log" 2>&1 &
  PIDS+=("$!")
  for _ in $(seq 1 100); do curl -sf -o /dev/null "http://127.0.0.1:$port/" && break; sleep 0.1; done
  curl -sf -o /dev/null "http://127.0.0.1:$port/" || { echo "kiln serve did not start for $1:" >&2; cat "$OUT/serve-$1.log" >&2; exit 1; }
  ARGS+=(--target "$1=$port")
}
serve examples examples
serve content examples/content
serve synthetic "$OUT/synthetic"
serve wizard "$OUT/empty"
serve longnames "$OUT/longnames"

# a scratch copy of the examples with a generated spec (a recipe, no parts) and a
# fake Godot that copies a valid GLB to the path the job asks for; it is served
# apart from the walkthrough targets, which keep their measured baselines
GEN="$OUT/generated"
cp -R examples "$GEN"
cat > "$GEN/specs/rock.toml" <<'TOML'
[asset]
id = "rock"
name = "Roca"
status = "layout"
budget_tris = 6000

[generator]
seed = 7

[[generator.ops]]
op = "sphere"
radius_m = [2, 5]
segments = 32

[[generator.ops]]
op = "noise"
amplitude = [0.1, 0.3]
size = 0.5

[[generator.ops]]
op = "facet"
TOML
cp "$GEN/godot/kits/harbor/crate.glb" "$GEN/fixture.glb"
cat > "$GEN/fake_godot.sh" <<'SH'
#!/bin/sh
for a; do job=$out; out=$a; done
d=$(dirname "$0")
glb=$(sed -n 's/.*"glb":"\([^"]*\)".*/\1/p' "$job")
cp "$d/fixture.glb" "$glb"
echo '{"size_m":[1,1,1],"min":[0,0,0],"tris":12,"godot":"fake"}' > "$out"
SH
chmod +x "$GEN/fake_godot.sh"
awk -v g="godot = \"$GEN/fake_godot.sh\"" '{print} /^godot_project = /{print g}' "$GEN/kiln.toml" > "$GEN/kiln.toml.new"
mv "$GEN/kiln.toml.new" "$GEN/kiln.toml"
GEN_PORT="$(free_port)"
"$KILN" serve --project "$GEN" --port "$GEN_PORT" >"$OUT/serve-generated.log" 2>&1 &
PIDS+=("$!")
for _ in $(seq 1 100); do curl -sf -o /dev/null "http://127.0.0.1:$GEN_PORT/" && break; sleep 0.1; done
curl -sf -o /dev/null "http://127.0.0.1:$GEN_PORT/" || { echo "kiln serve did not start for generated:" >&2; cat "$OUT/serve-generated.log" >&2; exit 1; }

set +e
node crates/kiln-cli/ui/tests/walkthrough.mjs --out "$OUT" ${ARGS[@]+"${ARGS[@]}"} $UPDATE
rc=$?
echo
echo "== generated recipe panel"
node crates/kiln-cli/ui/tests/app_generate.mjs --out "$OUT" --port "$GEN_PORT" --dir "$GEN"
rc_gen=$?
[ $rc -ne 0 ] || rc=$rc_gen
set -e
echo
echo "screenshots: $OUT/screenshots"
echo "metrics:     $OUT/metrics.json   (synthetic project: $OUT/synthetic)"
if [ $rc -eq 0 ]; then echo "OK: ui walkthrough passed"; else echo "FAIL: ui walkthrough failed (exit $rc)"; fi
exit $rc
