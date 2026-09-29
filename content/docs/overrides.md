+++
title = "Overrides and exclusions"
description = "Give one light its own color, radius, duration or flags, relight one interior cell, and leave lights, cells and plugins alone."
weight = 30

[extra]
kind = "guide"
+++

The multipliers treat every light of a kind the same way. Overrides pick lights and cells out by id
and give them values of their own. Both kinds of override go in
[`lightconfig.toml`](@/docs/lightconfig.md), or on the command line for one run.

Overrides and exclusions are keyed by [regular expressions](https://docs.rs/regex/latest/regex/#syntax),
matched without regard to case and anywhere in the id: `torch` matches `torch`, `Torch_256` and
`light_torch_blue`. Anchor them to be exact: `^torch$`.

## One light

```toml
[light_overrides.Light_Com_Candle_02]
red = 255
green = 128
blue = 64

[light_overrides.Torch_256]
red = 64
green = 128
blue = 255
hue = 220
value_mult = 0.75
radius = 254
duration = 1199.0

[light_overrides."^torch_128$"]
hue_mult = 0.3
red_mult = 1.1
radius_mult = 1.0
flag = ["CAN_CARRY", "PULSE_SLOW"]
```

Against Morrowind's own lights, those three give:

```text
LIGH "light_com_candle_02" from "Morrowind.esm": color [245, 140, 40, 0] -> [255, 128, 64, 0], radius 128 -> 153, duration 120 -> 300, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE)
LIGH "torch_256" from "Morrowind.esm": color [245, 140, 40, 0] -> [48, 96, 191, 0], radius 256 -> 254, duration 210 -> 1199, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE)
LIGH "torch_128" from "Morrowind.esm": color [245, 140, 40, 0] -> [154, 60, 46, 0], duration 210 -> 525, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW) -> LightFlags(CAN_CARRY | PULSE_SLOW)
```

The candle has exactly the color it was given, and the standard radius and duration multipliers
still apply to what the override leaves out. The second torch's fixed color is turned to hue 220°
and dimmed by a quarter. The third keeps its radius, `radius_mult = 1.0`, and has exactly the flags
listed.

A pattern that is not a bare TOML key, like `^torch_128$`, needs quotes. When several patterns
match a light, the first one listed is used and the others are ignored for that light.

| Key | Type | Does |
|---|---|---|
| `red`, `green`, `blue` | 0 to 255 | Replace the light's color, as the Construction Set shows it. All three or none |
| `hue` | 0 to 360 | Sets the hue, in degrees |
| `saturation`, `value` | 0.0 to 1.0 | Set the saturation or the brightness |
| `hue_mult`, `saturation_mult`, `value_mult` | number | Multiply the hue, saturation or brightness |
| `red_mult`, `green_mult`, `blue_mult` | number | Multiply one channel, after everything else. Results stay within 0 to 255 |
| `radius` | 0 to 4294967295 | Sets the radius |
| `radius_mult` | number | Multiplies the radius |
| `duration` | number | Sets the duration, truncated to a whole number. -1 never burns out |
| `duration_mult` | number | Multiplies the duration, unless it is -1 |
| `flag` | name, or list of names | Replaces all of the light's flags |

A key and its `_mult` form cannot both be set: `radius` and `radius_mult`, `duration` and
`duration_mult`, `hue` and `hue_mult`, and so on. `hue`, `saturation` and `value` outside their
range are clamped into it. Every multiplier must be a finite number.

### How the color is worked out

1. The starting color is the light's own, or `red`, `green` and `blue` if they are set.
2. Hue, saturation and value are each taken from the override: its `_mult` multiplies, or its fixed
   value replaces.
3. A component the override leaves out gets the global multiplier for the light's kind, such as
   `standard_value`, but only when the override has no `red`, `green` and `blue`. A fixed color is
   meant as the color, so it is not dimmed behind your back.
4. The result goes back to red, green and blue, and `red_mult`, `green_mult` and `blue_mult` apply
   last.

So `red = 255, green = 160, blue = 80` alone gives exactly that color, and adding `value_mult = 0.5`
halves its brightness.

### Radius, duration and flags

Radius and duration follow the same pattern: the override's multiplier, else its fixed value, else
the global multiplier. A fixed `duration` also applies to a light that never burns out, which is the
one way to give it a finite time; multipliers leave -1 alone.

`flag` replaces the whole set, including flags S3LightFixes would have removed, so
`flag = "FLICKER"` makes a light flicker even with `disable_flickering` on. `flag = "NONE"` clears
every flag. The names
are the Construction Set's light flags, in any case: `DYNAMIC`, `CAN_CARRY`, `NEGATIVE`,
`FLICKER`, `FIRE`, `OFF_BY_DEFAULT`, `FLICKER_SLOW`, `PULSE` and `PULSE_SLOW`. `NONE` cannot be
combined with any of them.

An override never applies to a negative light that `disable_negative_lights` has already turned off.

### On the command line

`--light` takes the pattern, `=`, and the keys as `key=value` pairs separated by commas. Flags are
joined with `|`. Single quotes keep the shell away from `$` and `|`:

```sh
s3lightfixes --light '^torch$=radius=400,red=255,green=160,blue=80' --light '^torch_128$=flag=CAN_CARRY|PULSE_SLOW'
```

```text
LIGH "torch" from "Morrowind.esm": color [245, 140, 40, 0] -> [255, 160, 80, 0], radius 256 -> 400, duration 210 -> 525, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | OFF_BY_DEFAULT | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE | OFF_BY_DEFAULT)
LIGH "torch_128" from "Morrowind.esm": color [245, 140, 40, 0] -> [140, 74, 46, 0], radius 128 -> 153, duration 210 -> 525, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW) -> LightFlags(CAN_CARRY | PULSE_SLOW)
```

Without the anchors, `torch` would also match `torch_128`, and as the first match it would be the
only one used.

Several overrides may also share one `--light`, separated by colons, so a pattern given on the
command line cannot contain `:` or `=`. Unknown keys and conflicting keys are errors there, before
anything runs:

```text
error: invalid value 'torch=radius=5,radius_mult=2' for '--light <LIGHT_OVERRIDES>': Key radius is mutually exclusive with radius_mult
```

Command-line overrides come after the ones in `lightconfig.toml`. One with the same pattern as a
file entry replaces it and moves to the end.

## One cell

```toml
[ambient_overrides."caius cosades' house"]
fog_density = 0.5

[ambient_overrides."caius cosades' house".ambient]
red = 64
green = 48
blue = 32
```

An ambient override matches interior cell ids, which are the cell names in lower case, like
`balmora, caius cosades' house`, and replaces what it sets:

| Key | Type | Replaces |
|---|---|---|
| `ambient` | `red`, `green`, `blue`, 0 to 255 | The cell's ambient light |
| `sunlight` | `red`, `green`, `blue`, 0 to 255 | The cell's sunlight |
| `fog` | `red`, `green`, `blue`, 0 to 255 | The fog color |
| `fog_density` | number | The fog density |

A color may instead be written as `hue`, `saturation` and `value`, all three, which older versions
used; it is converted to red, green and blue. Unlike light overrides, every ambient override that
matches a cell applies, in order, so a later one's values win.

On the command line, fields are separated by semicolons and colors are always red, green and blue:

```sh
s3lightfixes --ambient "caius cosades' house=ambient=red=64,green=48,blue=32;fog_density=0.5"
```

```text
CELL "balmora, caius cosades' house" from "Morrowind.esm": ambient [75, 65, 65, 0] -> [64, 48, 32, 0], fog_density 0.75 -> 0.5
```

Only interior cells that have ambient data can be overridden. The cell written to the plugin keeps
everything else from the plugin it came from, and carries none of its objects.

## Leaving things alone

```toml
excluded_ids = [
    # Every light or cell with "purple" in its id
    "purple",
    # Ids ending in glow
    "glow$",
]
```

`excluded_ids` matches light ids and interior cell ids. An excluded light or cell is not written to
the plugin at all, so the game uses whichever version it would have used without S3LightFixes.

`excluded_plugins` matches plugin file names. An excluded plugin is not read, so its lights neither
win nor lose against others: the last plugin that is read and has the light supplies it.

```toml
excluded_plugins = [
    "deleted_groundcover.omwaddon",
    "Clean_Argonian Full Helms Lore Integrated.ESP",
    "Baldurwind.omwaddon",
    "Crassified Navigation.omwaddon",
    "LuaMultiMark.omwaddon",
    "S3maphore.esp",
    "Toolgun.omwaddon",
    # Everything from OAAB
    "^OAAB",
]
```

The first seven are the built-in list, of plugins the reader cannot parse. Setting
`excluded_plugins` replaces that list, which is why the first run writes it into
`lightconfig.toml`: add to it there rather than starting a new one.

On the command line, `-x` and `-X` take comma-separated patterns and add them to the file's lists
for that run:

```sh
s3lightfixes -x 'purple,glow$' -X '^OAAB'
```
