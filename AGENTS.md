# AGENTS.md

## Repo shape
- Single Rust 2024 package: `s3lightfixes` exposes both `src/lib.rs` and the `s3lightfixes` binary (`src/main.rs` only calls `s3lightfixes::run()`).
- The binary generates `S3LightFixes.omwaddon` from the OpenMW config/content list; that generated plugin is intentionally gitignored.
- Main flow lives in `src/app.rs`; CLI schema in `src/light_args.rs`; TOML schema/merge/save logic in `src/light_config.rs`; per-light math in `src/light_processing.rs`; override parsers/types in `src/light_override.rs`; defaults and baked-in plugin exclusions in `src/default.rs`.

## Commands
- Full local verification: `cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings`.
- Focused test: `cargo test <test_name>` (there is only one package, so no `-p` needed).
- Run without GUI dialogs during manual checks: `S3L_NO_NOTIFICATIONS=1 cargo run -- ...` or pass `--no-notifications`.
- Non-writing checks: `cargo run -- --validate-config` validates config/regexes only; `cargo run -- --dry-run` also loads plugins and prints planned record changes without writing plugin/log/config files.
- Generate shell completions/manpage without needing an OpenMW install: `cargo run -- --generate-completion bash` or `cargo run -- --generate-manpage`.

## Runtime/config gotchas
- `--openmw-cfg/-c` accepts either a config file path or a directory containing `openmw.cfg`; if omitted, the code first checks `./openmw.cfg`, then platform defaults via `openmw-config`.
- `lightconfig.toml` is read/written next to the selected OpenMW user config unless a local config is supplied next to the executable; CLI values override TOML values, and list-style exclusions are merged rather than replaced.
- Output selection is not “any file path”: `--output/-o` must be an existing directory; without it, output goes to OpenMW `data-local`, falling back to cwd.
- `--auto-enable/-e` edits the selected OpenMW config to enable the generated plugin; avoid using it in tests unless that is exactly what you mean.
- Auto-enable backs up `openmw.cfg` to `openmw.cfg.s3lightfixes.bak` before editing; if the backup fails, it refuses to edit.
- `.cargo/config.toml` forces `git-fetch-with-cli = true`; dependency resolution for the git `tes3` dependency expects the `git` executable.

## Behavior worth preserving
- Only `CELL` and `LIGH` records are loaded from source plugins; generated output sorts objects and writes `lightconfig.log` next to the OpenMW config.
- Negative-light nulling defaults on but is user-configurable; flicker/pulse disabling, HSV/RGB overrides, and ambient/fog/sunlight overrides are user-facing compatibility behavior, not cosmetic refactors.
- `default::excluded_plugins()` contains specific known-bad plugin names with comments explaining parser failures; do not “clean up” those exclusions without reproducing the affected mods.
- CI delegates to `DreamWeave-MP/StroggForge/.github/workflows/rustGlobalBuild.yml@v27` and builds binary name `s3lightfixes`; keep release assumptions aligned with that workflow.
