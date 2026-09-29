+++
title = "Command line"
description = "Every s3lightfixes option, the environment variables it reads, completions and the manual page, and the exit codes."
weight = 50

[extra]
kind = "reference"
+++

```text
s3lightfixes [OPTIONS]
s3lightfixes --generate-completion <SHELL>
s3lightfixes --generate-manpage
```

Every option is optional. With none, a run reads the `openmw.cfg` it
[finds](@/docs/files.md#which-openmw-cfg), the settings in [`lightconfig.toml`](@/docs/lightconfig.md),
and writes the plugin. An option that also exists in `lightconfig.toml` overrides it for this run,
and is saved into the file when the run writes it.

## Where

| Option | Meaning |
|---|---|
| `-c`, `--openmw-cfg <PATH>` | The folder holding the `openmw.cfg` to read, or the path of a file named exactly `openmw.cfg`. Without it the config is [found](@/docs/files.md#found) |
| `-o`, `--output <DIR>` | The folder to write `S3LightFixes.omwaddon` into. It must exist |
| `-e`, `--auto-enable` | Add `content=S3LightFixes.omwaddon` to the user `openmw.cfg`, after backing it up. See [Enabling the plugin](@/docs/files.md#enabling-the-plugin) |

## What to change

The multipliers take a number, such as `0.5`; the switches take `true` or `false`.

| Option | Setting | Default |
|---|---|---|
| `-f`, `--no-flicker <BOOL>` | `disable_flickering` | `true` |
| `-p`, `--no-pulse <BOOL>` | `disable_pulse` | `false` |
| `--disable-negative-lights <BOOL>` | `disable_negative_lights` | `true` |
| `--standard-hue <NUMBER>` | `standard_hue` | 0.62 |
| `-s`, `--standard-saturation <NUMBER>` | `standard_saturation` | 0.8 |
| `-v`, `--standard-value <NUMBER>` | `standard_value` | 0.57 |
| `-r`, `--standard-radius <NUMBER>` | `standard_radius` | 1.2 |
| `-H`, `--colored-hue <NUMBER>` | `colored_hue` | 1.0 |
| `-S`, `--colored-saturation <NUMBER>` | `colored_saturation` | 0.9 |
| `--colored-value <NUMBER>` | `colored_value` | 0.7 |
| `-R`, `--colored-radius <NUMBER>` | `colored_radius` | 1.1 |
| `-M`, `--duration-mult <NUMBER>` | `duration_mult` | 2.5 |
| `-7`, `--classic` | `disable_interior_sun` | off |

`--standard-hue` has no short form, because `-h` is help. [What a run
changes](@/docs/what-it-changes.md#what-happens-to-a-light) explains each setting, and
[lightconfig.toml](@/docs/lightconfig.md#settings) repeats them with their keys.

```sh
s3lightfixes --no-flicker false --standard-radius 1.5 --duration-mult 1
```

## Which lights

| Option | Meaning |
|---|---|
| `-x`, `--excluded-ids <PATTERNS>` | Comma-separated patterns for light and cell ids to leave out. Added to `excluded_ids` |
| `-X`, `--excluded-plugins <PATTERNS>` | Comma-separated patterns for plugin file names not to read. Added to `excluded_plugins` |
| `--light <OVERRIDE>` | `pattern=key=value,key=value`: values for the lights the pattern matches. Repeat it, or separate several with `:` |
| `--ambient <OVERRIDE>` | `pattern=field=value;field=value`: ambient, sunlight, fog and fog density for the interior cells the pattern matches. Repeat it, or separate several with `:` |

Patterns are regular expressions, matched anywhere in the id and without regard to case. On the
command line a `-x` or `-X` pattern cannot contain a comma, and a `--light` or `--ambient` pattern
cannot contain `:` or `=`; `lightconfig.toml` has no such limits. [Overrides and
exclusions](@/docs/overrides.md) has the keys and examples:

```sh
s3lightfixes -x 'purple,glow$' --light '^torch$=radius=400,red=255,green=160,blue=80'
s3lightfixes --ambient "caius cosades' house=ambient=red=64,green=48,blue=32;fog_density=0.5"
```

## How to run

| Option | Meaning |
|---|---|
| `--dry-run [<BOOL>]` | Read everything, print the summary and every change, and write nothing: no plugin, log or settings |
| `--validate-config [<BOOL>]` | Read `lightconfig.toml` and the options, compile every pattern, print `Validated <path> successfully`, and stop. Plugins are not read |
| `-U`, `--update-light-config` | Save the settings of this run into `lightconfig.toml`. `save_config = true` in the file does it on every run |
| `-n`, `--no-notifications` | Print messages instead of showing dialogs |
| `-d`, `--debug` | Also print the settings, the OpenMW config and the plugin header to standard error |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print `s3lightfixes` and the version |

`--dry-run` and `--validate-config` without a value mean `true`, and `false` turns off a `dry_run` or
`validate_config` set in the file. They cannot be given together.

```sh
s3lightfixes --dry-run --standard-value 0.7 | grep '"torch"'
```

Messages go to a dialog box on Windows, macOS and Linux desktops, and are printed on Android. When
no dialog can be shown, such as on a Linux system without zenity, kdialog or yad, they are printed
too. The summary and the record lines always go to standard output; plugins that fail to load are
reported on standard error.

## Completions and the manual page

```sh
s3lightfixes --generate-completion bash > ~/.local/share/bash-completion/completions/s3lightfixes
s3lightfixes --generate-manpage > ~/.local/share/man/man1/s3lightfixes.1
```

`--generate-completion` takes `bash`, `elvish`, `fish`, `powershell` or `zsh`. Both print to
standard output and do nothing else: no config is read. They cannot be given together.

## Environment

| Variable | Effect |
|---|---|
| `S3L_NO_NOTIFICATIONS` | Set to anything: as `--no-notifications`, but never saved into `lightconfig.toml` |
| `S3L_DEBUG` | Set to anything: as `--debug`, but never saved |
| `OPENMW_CONFIG` | An `openmw.cfg` to read when there is no `--openmw-cfg` |
| `OPENMW_CONFIG_DIR` | Folders to look for `openmw.cfg` in, separated by `:`, or `;` on Windows |
| `OPENMW_GLOBAL_CONFIG_PATH` | Linux: the folder of OpenMW's global config, instead of `/etc/openmw` |
| `OPENMW_CONFIG_USING_FLATPAK` | Linux: set to anything to look for the Flatpak OpenMW's config |
| `OPENMW_FLATPAK_ID` | Linux: the Flatpak application id, instead of `org.openmw.OpenMW` |
| `XDG_CONFIG_HOME` | Linux: where the user config folder `openmw` is, instead of `~/.config` |

[Which openmw.cfg](@/docs/files.md#which-openmw-cfg) gives the order they are tried in.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Done. Also after a dry run, a successful check, `--help`, `--version`, and the generated completions and manual page |
| 1 | `lightconfig.toml` cannot be read, a pattern does not compile, `--output` is not a folder, `dry_run` and `validate_config` are both on, or a file could not be written |
| 2 | An option is wrong or unknown, or the plugin would be empty because no plugin supplied a record |
| 3 | A plugin's path has no file name. It should never happen |
| 4 | The load order has no content files |
| 127 | The `--openmw-cfg` path is wrong, or no `openmw.cfg` could be found or read |

For a pattern that does not compile, or a file that could not be written, standard error has the
details after `Error:`; the other failures show a message first.
