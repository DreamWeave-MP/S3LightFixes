+++
title = "Running"
description = "run, the file name constants, and the helpers for plugins, messages and errors: is_fixable_plugin, save_plugin, notification_box and to_io_error."
weight = 10

[extra]
kind = "api"
+++

## run

{{ api_signature(value="fn run() -> std::io::Result<()>") }}

The whole program. It parses the process's own arguments as [`LightArgs`](@/docs/api/settings.md#lightargs),
and then does what the [command line](@/docs/cli.md) says: reads the `openmw.cfg` chain, builds the
settings with [`LightConfig::get`](@/docs/api/settings.md#lightconfig-get), reads the plugins,
applies [`process_light`](@/docs/api/processing.md#process-light) to every light, writes the plugin
with [`save_plugin`](#save-plugin), enables it, and writes the log.

It returns an error when a file cannot be written or a pattern does not compile. Everything else
that goes wrong ends the process: bad arguments with code 2 and clap's message, and the other
failures with a [notification](#notification-box) and the exit code the command line reference
lists. The program's `main` is only:

```rust
fn main() -> std::io::Result<()> {
    s3lightfixes::run()
}
```

## Constants

{{ api_signature(value="const PLUGIN_NAME: &str = \"S3LightFixes.omwaddon\"") }}

The file name of the generated plugin, wherever it is written.

{{ api_signature(value="const DEFAULT_CONFIG_NAME: &str = \"lightconfig.toml\"") }}

The name the settings file is written under. It is found under any capitalization.

{{ api_signature(value="const LOG_NAME: &str = \"lightconfig.log\"") }}

The name of the log, written beside the settings.

## is_fixable_plugin

{{ api_signature(value="fn is_fixable_plugin(plug_path: &Path) -> bool") }}

Whether a run reads this file: it exists, its extension is `esp`, `esm`, `omwaddon` or `omwgame` in
any case, and its name without the extension is not `S3LightFixes` in any case, so the generated
plugin is never its own input. `excluded_plugins` is checked separately, by
[`LightConfig::is_excluded_plugin`](@/docs/api/settings.md#lightconfig-is-excluded-plugin).

```rust
use s3lightfixes::is_fixable_plugin;

fn main() -> std::io::Result<()> {
    let folder = std::env::temp_dir().join("s3lightfixes-fixable-example");
    std::fs::create_dir_all(&folder)?;
    for name in ["Better Lights.ESP", "S3LightFixes.esp", "music.omwscripts"] {
        std::fs::write(folder.join(name), [])?;
    }

    assert!(is_fixable_plugin(&folder.join("Better Lights.ESP")));
    assert!(!is_fixable_plugin(&folder.join("S3LightFixes.esp")));
    assert!(!is_fixable_plugin(&folder.join("music.omwscripts")));
    assert!(!is_fixable_plugin(&folder.join("Missing.esp")));

    std::fs::remove_dir_all(folder)
}
```

## save_plugin

{{ api_signature(value="fn save_plugin(output_dir: &PathBuf, generated_plugin: &mut Plugin) -> io::Result<()>") }}

Writes `generated_plugin` to [`PLUGIN_NAME`](#constants) in `output_dir`, as it is: the caller adds
the header and sorts the records, as `run` does. A missing `output_dir` is created. One that exists
but is not a folder is replaced by the current directory, with a warning on standard error.

Fails with the error from creating the folder, reading the current directory, or writing the file.

```rust
use s3lightfixes::{PLUGIN_NAME, Plugin, save_plugin};
use tes3::esp::{Header, Light, TES3Object};

fn main() -> std::io::Result<()> {
    let folder = std::env::temp_dir().join("s3lightfixes-save-example");
    let mut plugin = Plugin {
        objects: vec![
            TES3Object::Header(Header { num_objects: 1, ..Header::default() }),
            Light { id: "torch".to_owned(), ..Light::default() }.into(),
        ],
    };

    save_plugin(&folder, &mut plugin)?;

    let saved = Plugin::from_path(folder.join(PLUGIN_NAME))?;
    assert_eq!(saved.objects.len(), 2);
    std::fs::remove_dir_all(folder)
}
```

## notification_box

{{ api_signature(value="fn notification_box(title: &str, message: &str, no_notifications: bool)") }}

Shows `message` in a dialog titled `title` and waits for it to be closed. It prints `message` to
standard output instead when `no_notifications` is true, on Android, and when no dialog can be
shown, as on a Linux system without zenity, kdialog or yad.

## to_io_error

{{ api_signature(value="fn to_io_error<E: std::fmt::Display>(err: E) -> std::io::Error") }}

Wraps any displayable error as an `io::Error` of kind `InvalidData`, carrying its text. The
program uses it for TOML and configuration errors, so that everything it returns is an
`io::Error`.

```rust
use s3lightfixes::{OpenMWConfiguration, to_io_error};

fn main() {
    let missing = std::env::temp_dir().join("s3lightfixes-no-such-folder");
    let error = OpenMWConfiguration::new(Some(missing)).map_err(to_io_error).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    println!("{error}");
}
```

## Re-exports

{{ api_signature(value="pub use openmw_config::OpenMWConfiguration") }}

The parsed `openmw.cfg` chain [`LightConfig::get`](@/docs/api/settings.md#lightconfig-get) takes.
It comes from [openmw-config 1.1](https://docs.rs/openmw-config/1.1.0/openmw_config/).

{{ api_signature(value="pub use tes3::esp::Plugin") }}

A TES3 plugin, as [`save_plugin`](#save-plugin) takes it.
