+++
title = "Documentation"
description = "How to generate S3LightFixes.omwaddon for your load order, what it changes in every light, and how to tune it."
template = "docs/section.html"
page_template = "docs/page.html"
sort_by = "weight"

[extra]
docs_root = true
docs_project_name = "S3LightFixes"
docs_short_title = "S3LightFixes docs"
docs_project_path = "@/home/index.md"
docs_repository_url = "https://github.com/DreamWeave-MP/S3LightFixes/tree/main/content/docs"
docs_sidebar_label = "Documentation"
hide_child_cards = true
kind = "guide"
+++

S3LightFixes is a program that writes a plugin. It reads the lights in every plugin your
`openmw.cfg` loads, fixes them, and saves them into `S3LightFixes.omwaddon`, which you enable after
everything else. Nothing is downloaded but the program; the plugin is always made on your machine,
from your load order.

{% callout(kind="note", title="Which version these pages describe") %}
These pages describe the program on the default branch, which is what the development build and
the AUR package carry, and what releases are made from. 0.5.0 changed the command line: coming from
0.4.6 or earlier, [Upgrading from 0.4](@/docs/platforms.md#upgrading-from-0-4) lists how.
{% end %}

## Learn it

- **[Start here](@/docs/start-here.md)**: get the program, generate the plugin, enable it, and
  check what it did.
- **[What a run changes](@/docs/what-it-changes.md)**: which records it reads, which version of a
  light wins, and the arithmetic applied to each one.

## Tune it

- **[Overrides and exclusions](@/docs/overrides.md)**: one light's color, radius, duration or flags;
  one cell's ambient light and fog; lights and plugins to leave alone.
- **[Where files go](@/docs/files.md)**: which `openmw.cfg` it reads, where the plugin, the
  settings and the log are written, and what `--auto-enable` edits.

## Look it up

- **[Command line](@/docs/cli.md)**: every option, the environment variables, and the exit codes.
- **[lightconfig.toml](@/docs/lightconfig.md)**: every setting, its default, and when the file is
  written.
- **[Platforms, license and history](@/docs/platforms.md)**: what each download is, the AUR package,
  how releases are signed, what is tested, and what changed since the last release.
- **[Rust API](@/docs/api/_index.md)**: the library the program is built from.
