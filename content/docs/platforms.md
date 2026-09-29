+++
title = "Platforms, license and history"
description = "What each download is, the AUR package, how releases are signed, the license and when it changed, what CI tests, and what has changed since 0.4.6."
weight = 70

[extra]
kind = "reference"
+++

## Downloads

Every download is the same command-line program, built for one system.

| Download | For |
|---|---|
| `s3lightfixes-Windows-X64.zip` | Windows, x86-64 |
| `s3lightfixes-macOS-ARM64.zip` | macOS, Apple silicon |
| `s3lightfixes-macOS-X64.zip` | macOS, Intel |
| `s3lightfixes-Linux-X64.zip` | Linux, x86-64, glibc 2.28 or newer |
| `s3lightfixes-Portmaster-ARM64.zip` | ARM64 Linux handhelds, glibc 2.28 or newer. A program to run from a terminal or over SSH, not a PortMaster port |
| `s3lightfixes-Android-ARM64.zip` | Android 6 (API 23) or newer, ARM64. A program for a terminal such as Termux, not an app. Messages are printed, never shown as dialogs |

Each archive the current workflow builds holds `s3lightfixes` (`s3lightfixes.exe` on Windows),
`s3lightfixes-README.md`, `s3lightfixes-LICENSE`, and a Sigstore bundle. The Android and PortMaster builds are new since
0.4.6; that release has the four desktop archives.

On Android the user config is read from `/storage/emulated/0/Alpha3/config`, where OpenMW's
Android port keeps it; pass `--openmw-cfg` for anything else.

## Arch Linux

[`s3lightfixes-git`](https://aur.archlinux.org/packages/s3lightfixes-git) on the AUR builds the
default branch from source with `cargo build --release --all-features`, runs `cargo test`, and
installs `/usr/bin/s3lightfixes`. It is maintained by hand, not by this repository's release
workflow, so it follows the default branch rather than releases. [Start
here](@/docs/start-here.md#arch-linux) has the commands.

## Building it yourself

```sh
cargo build --release
```

The program has no Cargo features. Building needs the `git` executable, because the `tes3` library
comes from its Git repository and `.cargo/config.toml` tells Cargo to fetch it with `git`. For the
same reason S3LightFixes is not on crates.io, which does not accept Git dependencies.

## Signed releases

Each archive holds, beside the program, a Sigstore bundle for it, such as
`s3lightfixes-Linux-X64.bundle`, made by StroggForge's release workflow. It proves that workflow
built the program for this repository:

```sh
cosign verify-blob s3lightfixes \
  --bundle s3lightfixes-Linux-X64.bundle \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/DreamWeave-MP/StroggForge/\.github/workflows/rustGlobalBuild\.yml@' \
  --certificate-github-workflow-repository DreamWeave-MP/S3LightFixes
```

```text
Verified OK
```

Each GitHub release also links every archive's VirusTotal scan.

## License

MIT, from the first release after 0.4.6. Releases 0.4.44 to 0.4.6 are GPL-3.0-or-later; earlier
ones were published without a license.

## What is tested

Every push to the default branch, every tag and every pull request runs [StroggForge](https://github.com/DreamWeave-MP/StroggForge)'s Rust
workflow: the tests on Windows, Linux, and macOS on Apple silicon and Intel; Clippy at the pedantic
level with warnings as errors; `rustfmt`; and `cargo audit`, before anything is built for release.

The tests cover the settings and their merging, override parsing and precedence, the light
arithmetic, which version of a record wins, the masters, the cells, the log, the config paths, and
`--auto-enable`'s backup. Some run the program itself, on a scratch `openmw.cfg`.

## Since 0.4.6

The default branch has changed a great deal since 0.4.6, the latest release. The development build
and the AUR package have all of it; the next release will.

**Breaking**

- `-V` is `--version`. `--colored-value` no longer has a short form.
- `-i`, `--info` is gone: use `--version`. `-l`, `--write-log`, which wrote the whole plugin as text,
  is gone: every run writes `lightconfig.log` instead.
- Flag names in overrides are the Construction Set's, with underscores: `FLICKER_SLOW` and
  `PULSE_SLOW`. `FLICKERSLOW` and `PULSESLOW` are errors. `flag` now replaces the whole flag set
  and takes any of the nine flags, or a list of them.
- `--ambient` colors on the command line are `red`, `green` and `blue`. In `lightconfig.toml`,
  `hue`, `saturation` and `value` still work.
- `--openmw-cfg` takes a folder holding `openmw.cfg`, or a file named exactly `openmw.cfg`.
- Licensed MIT instead of GPL-3.0-or-later.

**Added**

- `red`, `green` and `blue` as a fixed color in light overrides, and `red_mult`, `green_mult` and
  `blue_mult`.
- `--dry-run` and `--validate-config`, also as `dry_run` and `validate_config` in the file.
- `--disable-negative-lights`: negative lights can be kept.
- `lightconfig.log`, with a summary and one line per changed record, also printed on every run.
- A backup of `openmw.cfg` before `--auto-enable` edits it.
- `--generate-completion` and `--generate-manpage`.
- Android and PortMaster builds.

**Fixed**

- `openmw.cfg` is found as OpenMW finds its root config, then the user's, on every system.
- Exclusions and overrides match ids and plugin names in any case.
- Infinite lights stay infinite, unless an override gives them a fixed duration.
- A plugin named `S3LightFixes` is never read as input, whatever its extension.
- Messages are printed when no dialog can be shown, and failures exit with a nonzero code on every
  system: an unreadable `lightconfig.toml` used to exit 0 on Linux and macOS.
- Saving rewrites `lightconfig.toml` under the name it was found with, instead of adding a second
  file on case-sensitive systems.
- A dry run piped into `head` no longer ends in an error.

## Older releases

The releases before 0.4.0, v0.1.1 to v0.3.3, from December 2024 to May 2025, were tagged with a `v`
in front. The changelog lists them with the rest, marked unverified, as it does 0.4.0 to 0.4.52:
their archives have older names, such as `windows-latest.zip`, that are not matched to a platform,
so none is recorded. They can still be downloaded from the
[GitHub releases page](https://github.com/DreamWeave-MP/S3LightFixes/releases). The lineage goes
further back: S3LightFixes succeeds
[Waza-lightfixes](https://github.com/glassmancody/waza_lightfixes), which followed vtastek's
Lightfixes.pl, a tes3cmd script. All three write a plugin that relights every mod in an
`openmw.cfg`; S3LightFixes is the one that needs nothing else installed.
