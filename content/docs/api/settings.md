+++
title = "Settings"
description = "LightArgs, the parsed command line; LightConfig, the settings a run uses, with get, is_excluded_id and is_excluded_plugin; and the default module."
weight = 20

[extra]
kind = "api"
+++

## LightArgs

{{ api_signature(value="pub struct LightArgs { pub openmw_cfg: Option<PathBuf>, pub use_classic: bool, pub output: Option<PathBuf>, … }") }}

The command line, as [clap](https://docs.rs/clap/4) parses it. It derives `clap::Parser`, so
`LightArgs::parse()` reads the process's arguments and `LightArgs::parse_from` any list, and
`LightArgs::command()` gives the `clap::Command` the completions and manual page are made from.
Each field is one [option](@/docs/cli.md):

| Field | Type | Option |
|---|---|---|
| `openmw_cfg` | `Option<PathBuf>` | `-c`, `--openmw-cfg` |
| `use_classic` | `bool` | `-7`, `--classic` |
| `output` | `Option<PathBuf>` | `-o`, `--output` |
| `auto_enable` | `bool` | `-e`, `--auto-enable` |
| `no_notifications` | `bool` | `-n`, `--no-notifications` |
| `debug` | `bool` | `-d`, `--debug` |
| `dry_run` | `Option<bool>` | `--dry-run` |
| `validate_config` | `Option<bool>` | `--validate-config` |
| `generate_completion` | `Option<clap_complete::Shell>` | `--generate-completion` |
| `generate_manpage` | `bool` | `--generate-manpage` |
| `disable_flickering` | `Option<bool>` | `-f`, `--no-flicker` |
| `disable_pulse` | `Option<bool>` | `-p`, `--no-pulse` |
| `disable_negative_lights` | `Option<bool>` | `--disable-negative-lights` |
| `standard_hue`, `standard_saturation`, `standard_value`, `standard_radius` | `Option<f32>` | `--standard-hue`, `-s`, `-v`, `-r` |
| `colored_hue`, `colored_saturation`, `colored_value`, `colored_radius` | `Option<f32>` | `-H`, `-S`, `--colored-value`, `-R` |
| `duration_mult` | `Option<f32>` | `-M`, `--duration-mult` |
| `excluded_ids` | `Vec<String>` | `-x`, `--excluded-ids` |
| `excluded_plugins` | `Vec<String>` | `-X`, `--excluded-plugins` |
| `light_overrides` | `Vec<(String, CustomLightData)>` | `--light` |
| `ambient_overrides` | `Vec<(String, CustomCellAmbient)>` | `--ambient` |
| `update_light_config` | `bool` | `-U`, `--update-light-config` |

`None` means the option was not given, so the setting comes from `lightconfig.toml`. `--light` and
`--ambient` are parsed as they are read, into [`CustomLightData`](@/docs/api/overrides.md#customlightdata)
and [`CustomCellAmbient`](@/docs/api/overrides.md#customcellambient), so a bad override fails the
parse.

```rust
use clap::Parser;
use s3lightfixes::LightArgs;

fn main() {
    let args = LightArgs::parse_from(["s3lightfixes", "-r", "1.5", "--light", "^torch$=radius=400"]);

    assert_eq!(args.standard_radius, Some(1.5));
    assert_eq!(args.light_overrides[0].0, "^torch$");
    assert_eq!(args.light_overrides[0].1.radius, Some(400));
    assert!(LightArgs::try_parse_from(["s3lightfixes", "--light", "torch=radius=5,radius_mult=2"]).is_err());
}
```

## LightConfig

{{ api_signature(value="pub struct LightConfig { pub standard_hue: f32, pub excluded_ids: Vec<String>, pub light_regexes: Vec<(Regex, CustomLightData)>, … }") }}

The settings of one run: [`lightconfig.toml`](@/docs/lightconfig.md) with the command line applied.
It is `Debug`, `Default`, `Serialize` and `Deserialize`, and serializes to the TOML file the program
writes. `Default` holds every [default](#the-default-module), no output folder, and empty lists and
tables apart from the built-in `excluded_plugins`.

The fields that are settings have the names and meanings of the
[keys in the file](@/docs/lightconfig.md#settings): `disable_interior_sun`, `disable_flickering`,
`disable_pulse`, `disable_negative_lights`, `auto_enable`, `no_notifications`, `debug`, `dry_run`,
`validate_config` and `save_config` are `bool`; the nine multipliers are `f32`; `excluded_ids` and
`excluded_plugins` are `Vec<String>`; `output_dir` is `Option<PathBuf>`; and

| Field | Type |
|---|---|
| `light_overrides` | `OrderedHashMap<String, CustomLightData>` |
| `ambient_overrides` | `OrderedHashMap<String, CustomCellAmbient>` |

`OrderedHashMap` is the `ordered_hash_map` crate's, which keeps the order the file lists the
overrides in.

Four more fields are not in the file. They hold the patterns compiled, without regard to case:

| Field | Type | Compiled from |
|---|---|---|
| `excluded_id_regexes` | `Vec<regex::Regex>` | `excluded_ids` |
| `excluded_plugin_regexes` | `Vec<regex::Regex>` | `excluded_plugins` |
| `light_regexes` | `Vec<(regex::Regex, CustomLightData)>` | `light_overrides` |
| `ambient_regexes` | `Vec<(regex::Regex, CustomCellAmbient)>` | `ambient_overrides` |

`LightConfig::get` fills them, and empties the lists and tables they came from. The matching
functions and [`process_light`](@/docs/api/processing.md#process-light) read only these.

### LightConfig::get

{{ api_signature(value="fn LightConfig::get(light_args: LightArgs, openmw_config: &OpenMWConfiguration) -> Result<LightConfig, io::Error>") }}

What a run does to build its settings:

1. Reads `lightconfig.toml` from the folder of the chain's user `openmw.cfg`, or starts from
   `Default` when there is none, and notes each key it does not read.
2. Applies `light_args` over it: the options given replace the file's values, `-x`, `-X`, `--light`
   and `--ambient` are added to its lists and tables, and `--classic` sets `disable_interior_sun`.
3. Picks the output folder, as [Where files go](@/docs/files.md#the-plugin) describes, unless this
   is a `--validate-config` run.
4. Writes `lightconfig.toml` when the file did not exist, `save_config` is set, or
   `update_light_config` is, and this is neither a dry run nor a check.
5. Reports the keys it did not read, as [Unknown keys](@/docs/lightconfig.md#unknown-keys)
   describes: a warning on standard error, or for a `--validate-config` run a notification and a
   failure. `save_log` and `auto_install`, which older versions wrote, are only mentioned, and only
   by a check.
6. Compiles the patterns.

Fails when the file cannot be read or written, when `dry_run` and `validate_config` are both on,
when a pattern does not compile, after a [notification](@/docs/api/running.md#notification-box)
for each bad pattern, and when a `--validate-config` run finds a key it does not know. It ends the
process, with exit code 1, when the file is not valid TOML or `--output` is not a folder.

```rust
use clap::Parser;
use s3lightfixes::{LightArgs, LightConfig, OpenMWConfiguration, to_io_error};

fn main() -> std::io::Result<()> {
    let folder = std::env::temp_dir().join("s3lightfixes-settings-example");
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join("openmw.cfg"), "content=Morrowind.esm\n")?;
    let openmw = OpenMWConfiguration::new(Some(folder.clone())).map_err(to_io_error)?;
    let args = LightArgs::parse_from(["s3lightfixes", "--dry-run", "-r", "1.5", "-x", "purple"]);

    let config = LightConfig::get(args, &openmw)?;

    assert!(config.dry_run);
    assert_eq!(config.standard_radius, 1.5);
    assert!(config.is_excluded_id("purple_512"));
    assert!(!folder.join("lightconfig.toml").exists());
    std::fs::remove_dir_all(folder)
}
```

### LightConfig::is_excluded_id

{{ api_signature(value="fn LightConfig::is_excluded_id(&self, record_id: &str) -> bool") }}

Whether any compiled `excluded_ids` pattern matches `record_id`, a light or cell id.

### LightConfig::is_excluded_plugin

{{ api_signature(value="fn LightConfig::is_excluded_plugin(&self, plugin_path: &Path) -> bool") }}

Whether any compiled `excluded_plugins` pattern matches the file name of `plugin_path`. A path
with no file name is never excluded.

```rust
use std::path::Path;

use s3lightfixes::LightConfig;

fn main() {
    let config: LightConfig = toml::from_str(r#"excluded_ids = ["^purple"]"#).unwrap();
    assert!(!config.is_excluded_id("purple_512"));

    let mut config = config;
    config.excluded_id_regexes.push(regex::RegexBuilder::new("^purple").case_insensitive(true).build().unwrap());
    config.excluded_plugin_regexes.push(regex::Regex::new("(?i)^oaab").unwrap());

    assert!(config.is_excluded_id("purple_512"));
    assert!(config.is_excluded_plugin(Path::new("/mods/OAAB_Data.esm")));
}
```

The first assertion is the point of the example: deserializing fills `excluded_ids`, but only
`LightConfig::get` compiles it.

## The default module

`s3lightfixes::default` holds the default of every setting that has one, as a function, which is
how serde fills a key the file leaves out.

| Function | Returns |
|---|---|
| `standard_hue()` | `0.62` |
| `standard_saturation()` | `0.8` |
| `standard_value()` | `0.57` |
| `standard_radius()` | `1.2` |
| `colored_hue()` | `1.0` |
| `colored_saturation()` | `0.9` |
| `colored_value()` | `0.7` |
| `colored_radius()` | `1.1` |
| `duration_mult()` | `2.5` |
| `disable_flicker()` | `true` |
| `disable_pulse()` | `false` |
| `disable_negative_lights()` | `true` |
| `auto_enable()` | `false` |
| `excluded_plugins()` | The [seven built-in plugin names](@/docs/lightconfig.md#excluded-plugins), as `Vec<String>` |

Each is `fn() -> f32`, `fn() -> bool`, or for `excluded_plugins`, `fn() -> Vec<String>`.

{{ api_signature(value="fn default::standard_radius() -> f32") }}

```rust
fn main() {
    assert_eq!(s3lightfixes::default::standard_radius(), 1.2);
    assert!(s3lightfixes::default::excluded_plugins().contains(&"S3maphore.esp".to_owned()));
}
```
