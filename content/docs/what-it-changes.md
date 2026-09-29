+++
title = "What a run changes"
description = "Which plugins and records S3LightFixes reads, which version of a light wins, the arithmetic applied to each light, the cells it touches, and what the plugin and the log contain."
weight = 20

[extra]
kind = "guide"
+++

## What it reads

The load order is every `content=` line in your `openmw.cfg` chain, in order. Each file is looked
up in the `data=` folders, then `data-local`, and as in OpenMW a later folder wins: when two hold a
file of the same name, the one OpenMW would load is the one read. Then it skips:

- files that are not `.esp`, `.esm`, `.omwaddon` or `.omwgame`, such as `.omwscripts`;
- its own plugin: any file named `S3LightFixes` with one of those extensions, in any case;
- plugins matching `excluded_plugins`, which by default lists
  [seven plugins it cannot read](@/docs/lightconfig.md#excluded-plugins);
- plugins that fail to load. Each prints a warning on standard error and the run goes on without
  it.

From each plugin it reads only two kinds of record, lights (`LIGH`) and cells (`CELL`), and it
reads the plugins in parallel.

## Which version of a light wins

When several plugins have a light with the same id, the last one in the load order wins, as it does
in OpenMW. S3LightFixes starts from that version, and ids are compared without regard to case. A
light whose id matches `excluded_ids` is skipped in every plugin, so no version of it is written
and OpenMW uses the one it would have used anyway.

Every other light goes into the plugin, changed or not. With the default settings almost all of
them change.

{% callout(kind="note", title="Cells and lights share ids") %}
A cell that a run changes claims its id before any light is considered. A light with the same id
as a changed cell, in any case, is then skipped as if it were excluded. Morrowind's own data has no
such pair.
{% end %}

## What happens to a light

In this order:

1. **Negative lights** go dark. A light with the `NEGATIVE` flag loses the flag and gets radius 0
   and color black, and nothing else below applies to it. `disable_negative_lights = false` keeps
   them as they are.
2. **Flicker** stops. `FLICKER` and `FLICKER_SLOW` are removed. `disable_flickering = false` keeps
   them.
3. **Pulse** stays, unless `disable_pulse = true`, which removes `PULSE` and `PULSE_SLOW`.
4. **Standard or colored.** The light's color is converted to hue, saturation and value. A hue from
   14° to 64°, inclusive, is the orange of fire and candles: a *standard* light. Anything else is a
   *colored* light, and that includes white and grey, whose hue is 0°.
5. **Color.** Hue, saturation and value are each multiplied by their multiplier for the light's
   kind, and converted back to red, green and blue.
6. **Radius** is multiplied by the kind's radius multiplier and rounded down.
7. **Duration** is multiplied by `duration_mult` and rounded toward zero. A duration of -1 means
   the light never burns out, and stays -1.

The multipliers, with their defaults:

| Setting | Standard lights | Colored lights |
|---|---|---|
| Hue | `standard_hue` = 0.62 | `colored_hue` = 1.0 |
| Saturation | `standard_saturation` = 0.8 | `colored_saturation` = 0.9 |
| Value | `standard_value` = 0.57 | `colored_value` = 0.7 |
| Radius | `standard_radius` = 1.2 | `colored_radius` = 1.1 |
| Duration | `duration_mult` = 2.5 | `duration_mult` = 2.5 |

So a standard light turns a little redder, less saturated and darker, and reaches a fifth further.
The torch from `Morrowind.esm` has the color `[245, 140, 40]`: hue 29.3°, saturation 0.84, value
0.96. After the multipliers that is hue 18.1°, saturation 0.67, value 0.55:

```text
LIGH "torch" from "Morrowind.esm": color [245, 140, 40, 0] -> [140, 74, 46, 0], radius 256 -> 307, duration 210 -> 525, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | OFF_BY_DEFAULT | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE | OFF_BY_DEFAULT)
```

A blue lantern, hue 210°, is a colored light: its hue stays, and it loses a little saturation and
brightness:

```text
LIGH "light_de_lantern_02" from "Morrowind.esm": color [0, 128, 255, 0] -> [18, 98, 178, 0], radius 256 -> 281, duration 150 -> 375, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE)
```

Duration is multiplied for every light that is not infinite. The help calls it the duration of
carried lights, the time a light burns once picked up; for the others the value is not used.

When a light matches a [light override](@/docs/overrides.md#one-light), the override's values
replace or refine steps 5 to 7 and may replace the flags. Steps 1 to 3 still come first, so an
override cannot bring back a negative light.

## What happens to a cell

Cells are only written when you ask for it. An interior cell with ambient data is written when:

- `--classic` or `disable_interior_sun = true` is set: its sunlight color becomes black. This is
  only for vtastek's shaders for OpenMW 0.47, which `--classic` is named after;
- or its id matches an [ambient override](@/docs/overrides.md#one-cell): the override's ambient,
  sunlight and fog colors and fog density replace the cell's.

The copy is the version from the last plugin that has the cell, without its objects and without its
water height, so loading it changes the cell's lighting and nothing else. Exterior cells are never
written.

## The plugin

`S3LightFixes.omwaddon` is an ordinary TES3 plugin: an ESP header, version 1.3, author `S3`,
description `Plugin generated by s3-lightfixes`. Its masters are the plugins that supplied at
least one of its records, in load order, with their sizes. For Morrowind with both expansions it
holds 676 lights and has three masters.

If no plugin supplies a record, say because every light is excluded, nothing is written: the run
stops with a message that the plugin would be empty, and exit code 2.

## The log

Each run prints a summary and one line per changed record, and writes the same text to
`lightconfig.log` next to `lightconfig.toml`, replacing the last run's. A dry run prints it with
`Dry run: no files written` in front, and writes no log.

```text
# S3LightFixes 0.5.0
# config: /home/you/.config/openmw/openmw.cfg
# output: /home/you/.local/share/openmw/data/S3LightFixes.omwaddon
# content files: 3
# loaded plugins: 3
# masters: 3
# changed cells: 1
# changed lights: 676
CELL "balmora, caius cosades' house" from "Morrowind.esm": ambient [75, 65, 65, 0] -> [64, 48, 32, 0], fog_density 0.75 -> 0.5
```

| Line | Means |
|---|---|
| `config` | The user `openmw.cfg` the run read, and whose folder holds the settings and the log |
| `output` | Where the plugin is written |
| `content files` | `content=` lines in the load order |
| `loaded plugins` | Plugins read, after the skips above |
| `masters` | Plugins that supplied a record |
| `changed cells`, `changed lights` | Records with at least one value changed |

A record line names the record type, its id as the plugin spells it, the plugin it came from, and
each value that changed: `color` and the cell colors as `[red, green, blue, 0]`, `radius`,
`duration`, `flags`, and `fog_density`. A light that did not change is in the plugin but not in the
log.
