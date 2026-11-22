# Context: k016-page-trust

Shared by every task of the plan. Read it before anything else. The main
session is the only writer.

## Decisions that shape the code

- `GET /api/session` gains `trusted` (bool). `false` means Kiln will not run
  Blender, Godot or Claude in the shown project; reads and edits still work.
- Untrusted launches answer 403 with a message naming `kiln trust --project <dir>`.
- An older server sends no `trusted`: the page treats `undefined` as trusted
  (no notice).

## Conventions

- Code and identifiers in English; UI text in Spanish; no emojis.
- UI: ES modules, `html` tagged template from `ui/app/html.js` escapes values.
- UI unit tests: `node --test 'crates/kiln-cli/ui/tests/unit/*.test.mjs'`.

## Key paths

- `crates/kiln-cli/src/serve.rs`: page HTTP handlers; `session()` builds `/api/session`.
- `crates/kiln-cli/src/core.rs`: `Page`, `Shared::show`.
- `crates/kiln-cli/ui/app/shell.js`: `watchCore` polls `/api/session`.

## Do not

- Touch files outside the task's Files cell. No git writes.

## Learned

- (empty)
