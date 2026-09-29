+++
title = "lightconfig.toml"
description = "Every setting in lightconfig.toml, its default and its command-line option, where the file lives, and when S3LightFixes writes it."
weight = 60

[extra]
kind = "reference"
+++

`lightconfig.toml` holds the settings S3LightFixes runs with. It lives in the same folder as your
user `openmw.cfg`, the one [the log's `config` line](@/docs/what-it-changes.md#the-log) names, and
is found there whatever its capitalization, so `lightConfig.toml` works too.

## When it is written

- **The first run** writes it when there is none, with every setting that run used.
- **Every run** writes it when it says `save_config = true`.
- **One run** writes it with `-U`, `--update-light-config`.

A dry run or a `--validate-config` run never writes it. When it is written, it holds the settings
the run actually used: the file's values, with any given on the command line in their place and the
command line's patterns added to its lists. The `S3L_NO_NOTIFICATIONS` and `S3L_DEBUG` environment
variables are not saved, but `-n` and `-d` are.

Written after a first run on Morrowind with both expansions, it reads:

```toml
disable_interior_sun = false
disable_flickering = true
disable_pulse = false
disable_negative_lights = true
auto_enable = false
no_notifications = false
debug = false
dry_run = false
validate_config = false
standard_hue = 0.62
standard_saturation = 0.8
standard_value = 0.57
standard_radius = 1.2
colored_hue = 1.0
colored_saturation = 0.9
colored_value = 0.7
colored_radius = 1.1
duration_mult = 2.5
excluded_plugins = [
    "deleted_groundcover.omwaddon",
    "Clean_Argonian Full Helms Lore Integrated.ESP",
    "Baldurwind.omwaddon",
    "Crassified Navigation.omwaddon",
    "LuaMultiMark.omwaddon",
    "S3maphore.esp",
    "Toolgun.omwaddon",
]
excluded_ids = []
output_dir = "/home/you/.local/share/openmw/data"
save_config = false

[light_overrides]

[ambient_overrides]
```

Every key is optional: one left out takes its default. Keys the program does not know are ignored,
misspelled ones included, and so is a top-level key written below a `[table]` header, because TOML
puts it inside that table. Keep settings above the override tables. A file that is not valid TOML,
or holds a value of the wrong type, stops the run with a message and exit code 1.

## Settings

### Lights

| Key | Default | Option | Does |
|---|---|---|---|
| `disable_flickering` | `true` | `-f`, `--no-flicker` | Remove `FLICKER` and `FLICKER_SLOW` from every light |
| `disable_pulse` | `false` | `-p`, `--no-pulse` | Remove `PULSE` and `PULSE_SLOW` from every light |
| `disable_negative_lights` | `true` | `--disable-negative-lights` | Turn every negative light off: no flag, radius 0, black |
| `standard_hue` | 0.62 | `--standard-hue` | Hue multiplier for standard lights, hue 14° to 64° |
| `standard_saturation` | 0.8 | `-s`, `--standard-saturation` | Saturation multiplier for standard lights |
| `standard_value` | 0.57 | `-v`, `--standard-value` | Brightness multiplier for standard lights |
| `standard_radius` | 1.2 | `-r`, `--standard-radius` | Radius multiplier for standard lights |
| `colored_hue` | 1.0 | `-H`, `--colored-hue` | Hue multiplier for every other light |
| `colored_saturation` | 0.9 | `-S`, `--colored-saturation` | Saturation multiplier for every other light |
| `colored_value` | 0.7 | `--colored-value` | Brightness multiplier for every other light |
| `colored_radius` | 1.1 | `-R`, `--colored-radius` | Radius multiplier for every other light |
| `duration_mult` | 2.5 | `-M`, `--duration-mult` | Duration multiplier for every light that is not infinite |

[What a run changes](@/docs/what-it-changes.md#what-happens-to-a-light) has the arithmetic.
`standard_radius` was once 2.0, which suited only vtastek's shaders; Modding-OpenMW.com's lists use
1.2, the default now.

### Cells

| Key | Default | Option | Does |
|---|---|---|---|
| `disable_interior_sun` | `false` | `-7`, `--classic` | Make every interior cell's sunlight black. Only for vtastek's shaders for OpenMW 0.47 |

`--classic` sets it for one run. A run that writes the file saves it, and from then on every run is
a classic one until you set it back to `false`.

### What to leave alone

| Key | Default | Option | Does |
|---|---|---|---|
| `excluded_ids` | `[]` | `-x`, `--excluded-ids` | Patterns for light and interior cell ids to leave out of the plugin |
| `excluded_plugins` | seven plugins | `-X`, `--excluded-plugins` | Patterns for plugin file names not to read |

The command-line options add to these lists; they do not replace them.
[Overrides and exclusions](@/docs/overrides.md#leaving-things-alone) explains both.

#### Excluded plugins

The default `excluded_plugins` names plugins the reader cannot parse: `deleted_groundcover.omwaddon`
has a moved reference it cannot resolve, `Clean_Argonian Full Helms Lore Integrated.ESP` has an
`FLTV` field in a cell, and `Baldurwind.omwaddon`, `Crassified Navigation.omwaddon`,
`LuaMultiMark.omwaddon`, `S3maphore.esp` and `Toolgun.omwaddon` have `LUAL` records. Setting the key
replaces the list.

### Overrides

| Table | Option | Holds |
|---|---|---|
| `[light_overrides."<pattern>"]` | `--light` | Color, radius, duration and flags for the lights the pattern matches |
| `[ambient_overrides."<pattern>"]` | `--ambient` | Ambient, sunlight and fog for the interior cells the pattern matches |

[Overrides and exclusions](@/docs/overrides.md) lists their keys. Overrides are tried in the order
the file lists them.

### The run itself

| Key | Default | Option | Does |
|---|---|---|---|
| `output_dir` | none | `-o`, `--output` | The folder to write `S3LightFixes.omwaddon` into |
| `auto_enable` | `false` | `-e`, `--auto-enable` | Add the plugin to the user `openmw.cfg` |
| `no_notifications` | `false` | `-n`, `--no-notifications` | Print messages instead of showing dialogs |
| `debug` | `false` | `-d`, `--debug` | Print the settings, the OpenMW config and the plugin header to standard error |
| `dry_run` | `false` | `--dry-run` | Print what would change and write nothing |
| `validate_config` | `false` | `--validate-config` | Check the settings and patterns, and stop |
| `save_config` | `false` | `-U` for one run | Write this file on every run |

`dry_run` and `validate_config` cannot both be `true`, unless the command line turns one of them
off.

`output_dir` needs care. The first run writes the folder it chose, `data-local` or the folder you
ran it from, into the file, and later runs keep writing there even if you move `data-local` or run
the program from elsewhere. Remove the line, or set it, when that changes. A folder named here that
does not exist is created; one given with `--output` must already exist.
[Where files go](@/docs/files.md#the-plugin) has the whole rule.
