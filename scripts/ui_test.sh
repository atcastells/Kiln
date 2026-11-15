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

set +e
node crates/kiln-cli/ui/tests/walkthrough.mjs --out "$OUT" ${ARGS[@]+"${ARGS[@]}"} $UPDATE
rc=$?
set -e
echo
echo "screenshots: $OUT/screenshots"
echo "metrics:     $OUT/metrics.json   (synthetic project: $OUT/synthetic)"
if [ $rc -eq 0 ]; then echo "OK: ui walkthrough passed"; else echo "FAIL: ui walkthrough failed (exit $rc)"; fi
exit $rc
