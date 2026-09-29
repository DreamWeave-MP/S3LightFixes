+++
title = "S3LightFixes"
description = "Generate a plugin that retunes every light in your OpenMW load order: color, radius, duration and flicker, from one command."

[taxonomies]
tags = ["OpenMW", "Morrowind", "Lighting", "Rust", "PortMaster"]
+++

S3LightFixes reads every plugin in your OpenMW load order and writes one more:
`S3LightFixes.omwaddon`, a fixed copy of every light record it found. Firelight turns softer and
reaches further, colored lights keep their color, flicker stops, negative lights go dark, and
carried lights burn longer. Enable it last, and every light in the game follows the same rules,
whichever mod it came from.

The program is what you download; the plugin is not. It is built from your load order, on your
machine, and has to be built again when your mods change.

{{ schematic(data_path="data/schematics/generate.json") }}

```sh
s3lightfixes
```

That is a whole run: it finds your `openmw.cfg`, reads the lights, writes the plugin to your
`data-local` folder, or the one you ran it in, and prints what it changed. Morrowind's torch, for one:

```text
LIGH "torch" from "Morrowind.esm": color [245, 140, 40, 0] -> [140, 74, 46, 0], radius 256 -> 307, duration 210 -> 525, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | OFF_BY_DEFAULT | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE | OFF_BY_DEFAULT)
```

Everything it does is a setting:

- **Multipliers**: hue, saturation, brightness and radius for firelight and for colored light,
  and one for duration.
- **Overrides**: a color, radius, duration or flags of your own for any light, and ambient light,
  sunlight and fog for any interior, chosen by regular expression.
- **Exclusions**: lights, cells and whole plugins to leave alone.
- **Enabling**: `--auto-enable` adds the plugin to your `openmw.cfg`, after backing the file up.

It descends from vtastek's Lightfixes.pl and wazabear's Waza-lightfixes, which did the same through
tes3cmd. S3LightFixes is one program with nothing else to install, on Windows, macOS, Linux,
Android and PortMaster handhelds.

## Documentation

- **[Start here](@/docs/start-here.md)**: get it, generate the plugin, enable it, and read what it
  changed.
- **[What a run changes](@/docs/what-it-changes.md)**: the arithmetic applied to every light.
- **[Overrides and exclusions](@/docs/overrides.md)**: one light, one cell, or none at all.
- **[Command line](@/docs/cli.md)** and **[lightconfig.toml](@/docs/lightconfig.md)**: every option
  and setting.

These pages describe the default branch, which the development build and the AUR package carry.
The latest release, 0.4.6, is older: [what has changed since](@/docs/platforms.md#since-0-4-6).
