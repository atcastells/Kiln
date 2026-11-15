# Context: k002-content-schema

Shared by every task of the plan. Read it before anything else. The main
session is the only writer.

## Where

- Work ONLY in this worktree: `~/Kiln/.claude/worktrees/k002-content-schema`
  (branch `k002-content-schema`, from `origin/main` at `ac0cc83`). Never touch
  `~/Kiln` itself (another checkout with unrelated uncommitted work) nor the
  worktree `k001-astra-validator`.
- Spec and plan: `.sdlc/k002-content-schema/{spec.md,plan.md}`.

## Decisions that shape the code

- Kiln assumes nothing about a concrete game. After this change no game name
  (ASTRA) may remain in non-test code: each project declares its content
  schema in its own `kiln.toml` under `[content.<set>]`.
- K001 (`crates/kiln-core/src/astra/`, `crates/kiln-cli/src/astra.rs`,
  `crates/kiln-cli/godot/astra_manifest.gd`, `[integrations.astra]`) is the
  behaviour to preserve, generalised. It is removed at the end (T9), without
  an alias.
- Findings keep the existing `Finding { severity, subject, code, message }`
  struct. Codes are `content_*`. Subject is `<set>:<file>#<id>`; for syntax
  errors `<set>:<file>` with line and column in the message.
- A project without `[content]` behaves exactly as before. An unknown key in
  a content schema is a finding (error), never silently ignored.
- Every read error (TOML, RON, JSON) is a finding, never a panic.
- No new dependencies (`ron`, `serde`, `serde_json`, `toml`, `glob` are there).
- Only the rules the K001 fixture and Astra need.

## Conventions

- Follow the style of the existing code in `crates/kiln-core/src` (read a
  neighbour file first). Code, identifiers and comments in English; comments
  only when not obvious; no emojis; no mentions of AI. User-facing finding
  messages are in Spanish, as the existing ones.
- Unit tests in the same file under `#[cfg(test)] mod tests`. Red test first,
  and say that you saw it fail.
- No `unwrap`/`expect`/panic outside tests.
- Tests needing files: a unique dir under `std::env::temp_dir()`; never write
  inside the repo; never shell out to `rm`.
- Run only your own tests (the command in your task). Incremental builds.
- About 100 new non-test lines per task; if it does not fit, stop and report.
- No git write commands.

## Baseline (T0)

- 77 tests pass, 1 ignored (the Godot one); clippy clean.
- `target/debug/kiln check --project examples` output saved at
  `/private/tmp/claude-501/-Users-acastells--t3-projects-astra/b0b93b11-6856-4049-a193-7257ad8af6dc/scratchpad/k002/baseline-examples.txt`.
- Astra's library to support: `/Users/acastells/.t3/projects/astra/content/astra/modules.ron`
  (a record `(modules: [ (key:.., kind: Access, size: (3, 2), connectors: [(name:.., side: East, offset: 0)], anchors: [(name:.., cell: (0, 0))], scene: "res://modules/access.tscn"), ... ])`).
  Astra's scenes: `/Users/acastells/.t3/projects/astra/godot/modules/*.tscn`
  (root metadata `astra_id = "sector:<key>"`, children `conn_<name>` with
  metadata `side`, `offset`, and `anchor_<name>` with metadata `cell`).

## Learned
