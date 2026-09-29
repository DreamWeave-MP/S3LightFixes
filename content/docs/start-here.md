+++
title = "Start here"
description = "Get S3LightFixes, generate S3LightFixes.omwaddon from your load order, enable it, and read what it changed."
weight = 10

[extra]
kind = "tutorial"
+++

You need OpenMW with a load order it already runs: an `openmw.cfg` with your `data=` folders and
`content=` files. S3LightFixes reads that file; it does not need the game running, or the launcher.

## Get it

### Download

Take the archive for your system from the [project page](@/home/index.md), or from GitHub's
[releases](https://github.com/DreamWeave-MP/S3LightFixes/releases), where `development` is the
latest build of the default branch these pages describe. Unzip it anywhere. It holds the program, `s3lightfixes` (`s3lightfixes.exe` on Windows), its readme and license, and a
signature. There is nothing to install.

- **macOS**: the builds are not notarized. If macOS will not open the program, run
  `xattr -d com.apple.quarantine s3lightfixes` in its folder, or allow it under System Settings,
  Privacy & Security.
- **Linux and macOS**: if the shell says permission denied, `chmod +x s3lightfixes`.

### Arch Linux

The AUR package [`s3lightfixes-git`](https://aur.archlinux.org/packages/s3lightfixes-git) builds the
default branch of this repository from source, runs its tests, and installs the program as
`/usr/bin/s3lightfixes`. With an AUR helper:

```sh
yay -S s3lightfixes-git
```

Without one:

```sh
git clone https://aur.archlinux.org/s3lightfixes-git.git
cd s3lightfixes-git
makepkg -si
```

It needs `git` and `cargo` to build. Being a `-git` package, it builds whatever the default branch
holds when you install or update it, not a tagged release. It installs only the program: generate
completions and the manual page yourself, as the [command line](@/docs/cli.md#completions-and-the-manual-page)
reference shows.

### From source

With Rust and `git` installed, from a clone of the repository:

```sh
cargo build --release
```

The program is `target/release/s3lightfixes`.

## Generate the plugin

Run the program. On Windows, double-click `s3lightfixes.exe`; elsewhere, run `s3lightfixes` from a
terminal, or double-click it if your file manager runs programs.

It finds your `openmw.cfg` the way OpenMW's own tools do, reads every plugin in its load order, and
writes `S3LightFixes.omwaddon` to your `data-local` folder, or to the folder you ran it from when
your config sets none. A message box says where. In a terminal it also prints one line for every
record it changed. For Morrowind with both expansions, that starts:

```text
# S3LightFixes 0.5.0
# config: /home/you/.config/openmw/openmw.cfg
# output: /home/you/.local/share/openmw/data/S3LightFixes.omwaddon
# content files: 3
# loaded plugins: 3
# masters: 3
# changed cells: 0
# changed lights: 676
LIGH "Light_Com_Candle_01" from "Bloodmoon.esm": color [245, 140, 40, 0] -> [140, 74, 46, 0], radius 128 -> 153, flags LightFlags(DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW) -> LightFlags(DYNAMIC | CAN_CARRY | FIRE)
```

and ends:

```text
S3LightFixes.omwaddon generated and saved in /home/you/.local/share/openmw/data
```

The first run also writes `lightconfig.toml` next to your `openmw.cfg`, holding every setting it
used. Edit it to change the next run; [lightconfig.toml](@/docs/lightconfig.md) explains each
setting.

If you use a portable OpenMW, or keep several configs, say which one:

```sh
s3lightfixes --openmw-cfg /games/total-overhaul
```

[Where files go](@/docs/files.md) covers how the config is found and where each file is written.

## Enable it

Open the OpenMW launcher, go to **Data Files**, tick `S3LightFixes.omwaddon`, and drag it to the
bottom of the list. It must load after every plugin with lights in it, or the plugins after it put
their own lights back.

If the plugin went to a folder OpenMW does not read, such as the one you ran the program from,
add that folder under **Data Directories** first. `data-local` is always read.
[This video](https://www.youtube.com/watch?v=xzq_ksVuRgc) shows the launcher's data directories and
content files, if they are new to you.

Or let the program do it: with `--auto-enable` it adds `content=S3LightFixes.omwaddon` to the end of
your user `openmw.cfg`, after copying that file to `openmw.cfg.s3lightfixes.bak`:

```sh
s3lightfixes --auto-enable
```

```text
S3LightFixes.omwaddon generated, enabled, and saved in /home/you/.local/share/openmw/data
```

## Check what it did

Every run writes the lines it printed to `lightconfig.log`, next to `lightconfig.toml`. Each line is
one record: its type, its id, the plugin it came from, and every value that changed, old then new.
[What a run changes](@/docs/what-it-changes.md) explains the values.

To see what a run would do without writing anything, not even the log:

```sh
s3lightfixes --dry-run
```

## Run it again

The plugin holds a copy of every light in your load order, made from the plugins you had when it
ran. Run S3LightFixes again whenever you add, remove or reorder mods, and whenever you change a
setting. It replaces `S3LightFixes.omwaddon` each time, and never reads its own plugin as input.
