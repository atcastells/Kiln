# Review: k016-page-trust

Three `reviewer` passes (bugs, security, conformance) over
`origin/k016-shared-core` + the working tree, consolidated by main. Every
Important below was reproduced with a red test or a code trace before fixing.

## Important (all fixed)

1. **conformance** `crates/kiln-cli/ui/app/api.js:20`: a 403 always showed the
   fixed cross-origin text, so the button never said `kiln trust --project`
   (R3). Fix: a 403 with an `error` body passes it through. Proven by an
   assertion added to `api.test.mjs` (red, then green).
2. **bugs** `crates/kiln-cli/ui/app/ui/feedback.js` (`trustNotice`): with no
   project open (`project: null`, `trusted: false`) the page showed the notice
   with an empty command. Fix: no notice without a project. Assertion added to
   `feedback.test.mjs` (red, then green).
3. **security** `crates/kiln-cli/src/main.rs` (`serve`): `kiln serve` with no
   `--project` shows the current folder, and `open_page` vouched for it. That
   is trust the person never gave (`kiln mcp` and `kiln trust` both need an
   explicit folder). Fix: `vouched = project_given || config`, threaded through
   `core::Opts`, `Hello::Open` (`#[serde(default)]`, so an older client is not
   vouched) and `open_page`. Assertion added to
   `an_untrusted_project_is_read_only_on_the_page` (red, then green).

## Nits

- Fixed: `shell.js` compared `innerHTML` with the generated string, so a path
  with quotes rewrote the live region on every poll; it now compares with the
  last string written.
- The cross-origin 403 now shows the server's own text ("solo desde la pagina
  de Kiln") instead of the fixed fallback.
- The path in the command is not shell-quoted (page notice, page 403 and the
  existing MCP message). A folder with spaces gives a command to fix by hand.
- `RUNS` is copied in the test; a new launching route must be added to both.
- The page's `/api/new` vouches a fresh folder only in memory, and does not
  write it to `trusted.json` as MCP `made` does. This is out of scope in the spec.
- A vouch is tied to the path, not to the `--config` used. This is the same
  granularity as K014.

## Unconfirmed

- Any local process can still POST to the page with `x-kiln: 1`. This was
  already true before the change. The control covers MCP tools and other
  websites, not processes run by the same user.
- `/api/search` (curl to fixed hosts) is not gated, the same as MCP `search`.

## Verification

- `tests/core.rs` baseline on the base before T1: 4/4. The T1 red run failed
  because `trusted` was missing. After the fixes it passes 5/5.
- Final battery: see the PR description.
- `scripts/ui_test.sh`: one baseline check is named with a count ("4 dimmed"),
  and today it gives "7 dimmed". The same red shows on `origin/k016-shared-core`
  without this change, so it already existed.
