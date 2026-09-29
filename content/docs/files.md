+++
title = "Where files go"
description = "How S3LightFixes finds openmw.cfg, where it writes the plugin, lightconfig.toml and lightconfig.log, and what --auto-enable changes."
weight = 40

[extra]
kind = "guide"
+++

A run reads one `openmw.cfg` chain and writes up to four files:

| File | Where | When |
|---|---|---|
| `S3LightFixes.omwaddon` | [The output folder](#the-plugin) | Every run but a dry run or a check |
| `lightconfig.toml` | Beside the user `openmw.cfg` | [The first run, and when asked](@/docs/lightconfig.md#when-it-is-written) |
| `lightconfig.log` | Beside the user `openmw.cfg` | Every run but a dry run or a check |
| `openmw.cfg.s3lightfixes.bak` | Beside the user `openmw.cfg` | With `--auto-enable`, before it edits the file |

## Which openmw.cfg

### Given on the command line

```sh
s3lightfixes --openmw-cfg /games/total-overhaul
s3lightfixes --openmw-cfg /games/total-overhaul/openmw.cfg
```

`-c`, `--openmw-cfg` takes a folder that holds an `openmw.cfg`, or the path of a file named exactly
`openmw.cfg`. A relative path starts from the folder you run the program in. Anything else stops
the run with a message saying what was wrong, and exit code 127.

Use it for a portable OpenMW, or when a launcher keeps its own config, as `omw` does.

### Found

Without `--openmw-cfg`, the first of these that exists is read:

1. The file `OPENMW_CONFIG` names.
2. An `openmw.cfg` in one of the folders `OPENMW_CONFIG_DIR` lists, separated by `:`, or `;` on
   Windows.
3. An `openmw.cfg` in the same folder as the `s3lightfixes` program.
4. On Linux, OpenMW's global config: `/etc/openmw/openmw.cfg`, `/app/etc/openmw/openmw.cfg` in a
   Flatpak, or the folder `OPENMW_GLOBAL_CONFIG_PATH` names.
5. Your own `openmw.cfg`:

| System | User config |
|---|---|
| Windows | `Documents\My Games\OpenMW\openmw.cfg` |
| macOS | `~/Library/Preferences/openmw/openmw.cfg` |
| Linux | `~/.config/openmw/openmw.cfg`, or under `$XDG_CONFIG_HOME` when it is set |
| Linux, Flatpak OpenMW | `~/.var/app/org.openmw.OpenMW/config/openmw/openmw.cfg` |
| Android | `/storage/emulated/0/Alpha3/config/openmw.cfg` |

S3LightFixes treats OpenMW as Flatpak'd when `OPENMW_CONFIG_USING_FLATPAK` is set, or when it runs
inside a Flatpak itself. `OPENMW_FLATPAK_ID` changes the application id from `org.openmw.OpenMW`.

If none exists, the run stops with a message listing where it looked, and exit code 127.

### The chain

The file found first is the root. Like OpenMW, S3LightFixes follows its `config=` lines, usually
`config="?userconfig?"`, and reads each file they name, in order. The last one is the *user*
`openmw.cfg`: its folder holds `lightconfig.toml` and `lightconfig.log`, and it is the file
`--auto-enable` edits. A root with no `config=` line is its own user config.

The load order is every `content=` line in the chain, and the plugins are found in every `data=`
folder in the chain, then `data-local`.

## The plugin

The plugin is always named `S3LightFixes.omwaddon`. It is written to the first of:

1. The folder given with `-o`, `--output`. It must exist; a relative path starts from the folder you
   run the program in. If it does not exist, the run stops with exit code 1.
2. `output_dir` in `lightconfig.toml`. A missing folder is created. If it is a file, the plugin goes
   to the folder you ran the program in, with a warning.
3. `data-local`, if the chain sets one. It is created if missing.
4. The folder you ran the program in.

The first run records its choice as `output_dir` in `lightconfig.toml`, so later runs write to the
same folder until you change or remove that line.

Before writing, S3LightFixes deletes any `S3LightFixes.omwaddon` in `data-local`. OpenMW reads
`data-local` after every other data folder, so an old copy there would hide the new one wherever
it was written.

OpenMW only finds the plugin in a folder it reads. `data-local` always is; any other output folder
needs a `data=` line, or an entry under **Data Directories** in the launcher.

## Enabling the plugin

`-e`, `--auto-enable` enables the plugin after writing it:

1. If any file in the chain already has `content=S3LightFixes.omwaddon`, nothing changes.
2. Otherwise the user `openmw.cfg` is copied to `openmw.cfg.s3lightfixes.bak`, replacing an older
   backup. If the copy fails, the plugin is not enabled, and a message says why.
3. `content=S3LightFixes.omwaddon` is added at the end of the file, and the file is written back.

It adds no `data=` line, so the plugin must already be somewhere OpenMW reads.

{% callout(kind="warning", title="Comments at the end of openmw.cfg") %}
Rewriting the user `openmw.cfg` keeps its settings and the comments above them, but drops comments
and blank lines after its last setting. That is a bug in openmw-config, the library that writes the
file. The backup keeps them.
{% end %}

## Settings and the log

`lightconfig.toml` and `lightconfig.log` are in the user config's folder: the folder of the file
the log's `config` line names. The log is replaced on every run that writes a plugin.
[lightconfig.toml](@/docs/lightconfig.md) and [the log](@/docs/what-it-changes.md#the-log) describe
their contents.
