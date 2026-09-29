+++
title = "Rust API"
description = "The s3lightfixes library the program is built from: its settings, override types, light arithmetic and plugin helpers."
template = "docs/section.html"
page_template = "docs/page.html"
sort_by = "weight"
weight = 90

[extra]
kind = "api"
hide_child_cards = true
+++

The `s3lightfixes` package is a library with the program on top: `src/main.rs` only calls
[`run`](@/docs/api/running.md#run). Everything the program does is public, so a tool can read the
same settings, apply the same arithmetic to its own lights, or run the whole thing.

It is not on crates.io, because it depends on the `tes3` library from Git, which crates.io does not
accept. Depend on the repository, and on `tes3` for the record types:

```toml
[dependencies]
s3lightfixes = { git = "https://github.com/DreamWeave-MP/S3LightFixes" }
tes3 = { git = "https://github.com/Greatness7/tes3", branch = "main", features = ["esp"] }
```

S3LightFixes's own `.cargo/config.toml` sets `net.git-fetch-with-cli = true`, so Cargo fetches
`tes3` with the `git` program; your project may want the same.

One light, with the program's default settings:

```rust
use s3lightfixes::{LightConfig, process_light};
use tes3::esp::{Light, LightData, LightFlags};

fn main() {
    let config = LightConfig::default();
    let mut torch = Light {
        id: "torch".to_owned(),
        data: LightData {
            radius: 256,
            time: 210,
            color: [245, 140, 40, 0],
            flags: LightFlags::DYNAMIC | LightFlags::CAN_CARRY | LightFlags::FIRE | LightFlags::FLICKER_SLOW,
            ..LightData::default()
        },
        ..Light::default()
    };

    let changes = process_light(&config, &mut torch);

    assert_eq!(torch.data.color, [140, 74, 46, 0]);
    assert_eq!(torch.data.radius, 307);
    assert_eq!(torch.data.time, 525);
    assert_eq!(torch.data.flags, LightFlags::DYNAMIC | LightFlags::CAN_CARRY | LightFlags::FIRE);
    println!("{}", changes.join(", "));
}
```

| Page | Covers |
|---|---|
| [Running](@/docs/api/running.md) | `run`, the file name constants, and the helpers for plugins, messages and errors |
| [Settings](@/docs/api/settings.md) | `LightArgs`, `LightConfig`, and the `default` module |
| [Overrides](@/docs/api/overrides.md) | `CustomLightData`, `CustomCellAmbient`, their color and flag types, and their errors |
| [Light arithmetic](@/docs/api/processing.md) | `process_light` and `light_to_hsv` |

## Made for the program

The library is the program taken apart, not a library designed first. Some of it acts like a
program:

- [`LightConfig::get`](@/docs/api/settings.md#lightconfig-get) and [`run`](@/docs/api/running.md#run)
  end the process on some errors, with the exit codes the [command line](@/docs/cli.md#exit-codes)
  lists, rather than returning them.
- [`notification_box`](@/docs/api/running.md#notification-box) opens a dialog unless told not to.
- The regular expressions in `LightConfig` are compiled only by `LightConfig::get`. A config built
  any other way has empty pattern lists until you fill them.

The crate re-exports `openmw_config::OpenMWConfiguration` and `tes3::esp::Plugin`, the types its
functions take.
