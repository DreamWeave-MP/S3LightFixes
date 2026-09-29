# S3LightFixes

Generate a plugin that retunes every light in your OpenMW load order.

S3LightFixes reads every plugin your `openmw.cfg` loads and writes one more,
`S3LightFixes.omwaddon`: a fixed copy of every light record. Firelight turns softer and reaches
further, colored lights keep their color, flicker stops, negative lights go dark, and carried
lights burn longer. Every multiplier is a setting, and any light or interior cell can be given
values of its own.

The plugin is not something you download. It is built from your load order, on your machine, and
built again when your mods change. S3LightFixes descends from vtastek's Lightfixes.pl and
wazabear's Waza-lightfixes, and needs nothing else installed.

**Documentation, downloads and the Rust API reference: <https://dreamweave-mp.github.io/S3LightFixes/>**

## Install

Download the build for your system from the
[releases](https://github.com/DreamWeave-MP/S3LightFixes/releases): Windows, macOS, Linux, Android
and PortMaster. The [development build](https://github.com/DreamWeave-MP/S3LightFixes/releases/tag/development)
is the default branch, which the documentation describes.

On Arch Linux, the AUR package [`s3lightfixes-git`](https://aur.archlinux.org/packages/s3lightfixes-git)
builds the default branch:

```sh
yay -S s3lightfixes-git
```

Or build it from a clone, with Rust and `git`:

```sh
cargo build --release
```

## Use

```sh
s3lightfixes
```

That finds your `openmw.cfg`, writes `S3LightFixes.omwaddon` to your `data-local` folder, or the
folder you ran it in, and prints every change. Enable the plugin last in your load order, or let
the program add it:

```sh
s3lightfixes --auto-enable
s3lightfixes --openmw-cfg /games/total-overhaul   # a portable or second install
s3lightfixes --dry-run                             # print the changes, write nothing
```

Settings live in `lightconfig.toml` beside your `openmw.cfg`, written on the first run.

## As a library

The program is a library with a `main` on top. It is not on crates.io, because it depends on
`tes3` from Git:

```toml
[dependencies]
s3lightfixes = { git = "https://github.com/DreamWeave-MP/S3LightFixes" }
```

```rust
let changes = s3lightfixes::process_light(&s3lightfixes::LightConfig::default(), &mut light);
```

## Where to read next

- [Start here](https://dreamweave-mp.github.io/S3LightFixes/docs/start-here/): a first run
- [What a run changes](https://dreamweave-mp.github.io/S3LightFixes/docs/what-it-changes/): the
  arithmetic applied to every light
- [Overrides and exclusions](https://dreamweave-mp.github.io/S3LightFixes/docs/overrides/)
- [Command line](https://dreamweave-mp.github.io/S3LightFixes/docs/cli/) and
  [lightconfig.toml](https://dreamweave-mp.github.io/S3LightFixes/docs/lightconfig/)
- [Rust API](https://dreamweave-mp.github.io/S3LightFixes/docs/api/)

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -W clippy::pedantic -D warnings
cargo test --all-features
```

The site in `content/` is a
[DreamWeave Mod Template](https://github.com/DreamWeave-MP/DreamWeave-Mod-Template) site; preview it
with `zola serve`.

## License

S3LightFixes is released under the [MIT License](LICENSE).
