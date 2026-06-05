use std::path::PathBuf;

use clap::Parser;
use clap_complete::Shell;

use crate::default;

#[derive(Parser, Debug)]
#[command(
    name = "s3lightfixes",
    author,
    version,
    about = "A tool for modifying light values globally across an OpenMW installation.\nPlease note that arguments provided here, which also exist in lightConfig.toml, will override any values in lightConfig.toml when used.\nAdditionally, if the lightConfig.toml does not exist, the used values will be saved into the new lightConfig.toml."
)]
// CLI flags are naturally boolean. Turning these into enums would make the Rust type prettier
// while making the command-line interface and clap mapping pointlessly dishonest.
#[allow(clippy::struct_excessive_bools)]
pub struct LightArgs {
    /// Directory containing openmw.cfg.
    /// By default, uses `OpenMW` root config discovery, falling back to the default user config.
    /// These paths are defined by:
    /// <https://openmw.readthedocs.io/en/latest/reference/modding/paths.html>
    /// May also be the literal path to an openmw.cfg file; the filename must be exactly openmw.cfg.
    #[arg(short = 'c', long = "openmw-cfg")]
    pub openmw_cfg: Option<PathBuf>,

    /// Enables classic mode using vtastek shaders.
    /// ONLY for openmw 0.47. Relevant shaders can be found in the `OpenMW` discord:
    /// <https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148>
    #[arg(short = '7', long = "classic")]
    pub use_classic: bool,

    /// Output directory.
    /// The plugin may be saved to any location, but its name will always be `S3Lightfixes.omwaddon`.
    /// Accepts relative and absolute terms.
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Whether to automatically enable the output plugin in openmw.cfg.
    /// Disabled by default.
    /// Typically lightfixes is ran under momw-configurator, making this param
    /// unnecessary for many users.
    #[arg(short = 'e', long = "auto-enable")]
    pub auto_enable: bool,

    /// If used, print to stdout instead of using native GUI dialogs.
    /// Not available on android.
    #[arg(short = 'n', long = "no-notifications")]
    pub no_notifications: bool,

    /// Output debugging information during lightfixes generation
    /// Primarily displays output related to the openmw.cfg being used for generation
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    /// Validate config and source plugins, print planned changes, but do not write files.
    #[arg(
        long = "dry-run",
        conflicts_with = "validate_config",
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL"
    )]
    pub dry_run: Option<bool>,

    /// Validate lightconfig.toml, CLI overrides, and regexes without generating a plugin.
    #[arg(
        long = "validate-config",
        conflicts_with = "dry_run",
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL"
    )]
    pub validate_config: Option<bool>,

    /// Generate shell completion script to stdout
    #[arg(long, value_name = "SHELL", conflicts_with = "generate_manpage")]
    pub generate_completion: Option<Shell>,

    /// Generate roff manpage to stdout
    #[arg(long, conflicts_with = "generate_completion")]
    pub generate_manpage: bool,

    /// Whether to disable flickering lights during lightfixes generation
    #[arg(short = 'f', long = "no-flicker")]
    pub disable_flickering: Option<bool>,

    /// Whether to disable pulsing lights during lightfixes generation
    #[arg(short = 'p', long = "no-pulse")]
    pub disable_pulse: Option<bool>,

    /// Whether to null negative lights during lightfixes generation
    #[arg(long = "disable-negative-lights")]
    pub disable_negative_lights: Option<bool>,

    #[arg(
        long = "standard-hue",
        help = &format!("For lights in the orange range, multiply their HSV hue by this value.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.\nThis argument has no short form due to a conflict with -h.", default::standard_hue())
    )]
    pub standard_hue: Option<f32>,

    #[arg(
        short = 's',
        long = "standard-saturation",
        help = &format!("For lights in the orange range, multiply their HSV saturation by this amount.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::standard_saturation())
    )]
    pub standard_saturation: Option<f32>,

    #[arg(
        short = 'v',
        long = "standard-value",
        help = &format!("For lights in the orange range, multiply their HSV value by this amount.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::standard_value())
    )]
    pub standard_value: Option<f32>,

    #[arg(
        short = 'r',
        long = "standard-radius",
        help = &format!("For lights in the orange range, multiply their radius by this value.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::standard_radius())
    )]
    pub standard_radius: Option<f32>,

    #[arg(
        short = 'H',
        long = "colored-hue",
        help = &format!("For lights that are red, purple, blue, green, or yellow, multiply their HSV hue by this value.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::colored_hue())
    )]
    pub colored_hue: Option<f32>,

    #[arg(
        short = 'S',
        long = "colored-saturation",
        help = &format!("For lights that are red, purple, blue, green, or yellow, multiply their HSV saturation by this amount.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::colored_saturation())
    )]
    pub colored_saturation: Option<f32>,

    #[arg(
        long = "colored-value",
        help = &format!("For lights that are red, purple, blue, green, or yellow, multiply their HSV value by this amount.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::colored_value())
    )]
    pub colored_value: Option<f32>,

    #[arg(
        short = 'R',
        long = "colored-radius",
        help = &format!("For lights that are red, purple, blue, green, or yellow, multiply their radius by this value.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::colored_radius())
    )]
    pub colored_radius: Option<f32>,

    #[arg(
        short = 'M',
        long = "duration-mult",
        help = &format!("Multiplies the duration of all carryable lights.\nIf this argument is not used, the value will be derived from lightConfig.toml or use the default value of {}.", default::duration_mult())
    )]
    pub duration_mult: Option<f32>,

    #[arg(
        short = 'x',
        long = "excluded-ids",
        help = &format!("List of Regex patterns of light recordIds to exclude. This setting is *merged* onto values defined by lightconfig.toml.\nIf this argument is not used, the value will be derived from lightConfig.toml."),
        value_delimiter = ',',
    )]
    pub excluded_ids: Vec<String>,

    #[arg(
        short = 'X',
        long = "excluded-plugins",
        help = &format!("List of Regex patterns of plugins to exclude. This setting is *merged* onto values defined by lightconfig.toml.\nIf this argument is not used, the value will be derived from lightConfig.toml."),
        value_delimiter = ',',
    )]
    pub excluded_plugins: Vec<String>,

    #[arg(
        long = "light",
        value_parser = crate::light_override::parse_light_override,
        value_delimiter = ':',
        help = &format!(
     "Colon-separated list of regexes to light values.
     May be specified multiple times instead of as a separated list.
     Light color values may use fixed RGB fields (`red`, `green`, `blue`), HSV fixed fields (`hue`, `saturation`, `value`), HSV multipliers (`hue_mult`, `saturation_mult`, `value_mult`), and RGB multipliers (`red_mult`, `green_mult`, `blue_mult`).
     EG:
     --light \"Torch_001=radius=255,red=255,green=128,blue=64,hue=220,blue_mult=0.5,duration=1200,flag=FLICKER_SLOW\" --light \"Torch_002=radius_mult=2.0,hue_mult=1.3,red_mult=1.1,duration_mult=5.0,flag=CAN_CARRY|PULSE_SLOW\"
     OR
     --light \"Torch_001=radius=255,red=255,green=128,blue=64,hue=220,blue_mult=0.5,duration=1200,flag=FLICKER_SLOW:Torch_002=radius_mult=2.0,hue_mult=1.3,red_mult=1.1,duration_mult=5.0,flag=CAN_CARRY|PULSE_SLOW\"
     RGB color components are 0-255, matching TES3/Construction Set values. Radius and duration are u32 (can be very big).
     `flag` may include: NONE, DYNAMIC, CAN_CARRY, NEGATIVE, FLICKER, FIRE, OFF_BY_DEFAULT, FLICKER_SLOW, PULSE, PULSE_SLOW. Separate multiple flags with `|`. This replaces the source light's full flag set.
     Color precedence: fixed RGB, when present, replaces the source RGB as the base color and disables global HSV fallback for missing HSV components; without fixed RGB, missing HSV components still use the standard/colored global HSV multipliers. HSV fixed fields/multipliers adjust the selected base color per component, fixed and multiplier forms for the same HSV component are mutually exclusive, and RGB multipliers are always applied last."),
    )]
    pub light_overrides: Vec<(String, crate::CustomLightData)>,

    #[arg(
        long = "ambient",
        value_parser = crate::light_override::parse_ambient_override,
        value_delimiter = ':',
        help = &format!(
            "
            Colon-separated list of cell id regexes, to the corresponding ambient data.
            `sunlight`, `ambient`, `fog`, and `fog_density` are available parameters.
            Values are provided as fixed RGB values, no multipliers.
            RGB color components are 0-255, matching TES3/Construction Set values.
            Each field of cell ambient data is separated by a semicolon, as below:
            --ambient \"caius cosades\' house=sunlight=red=255,green=255,blue=255;ambient=red=64,green=48,blue=32\"
            "
        )
    )]
    pub ambient_overrides: Vec<(String, crate::CustomCellAmbient)>,

    #[arg(
        short = 'U',
        long,
        help = &format!("Force-saves the light config on this run. Note that this parameter does not merge into lightConfig.toml like others, and must be manually set there.")
    )]
    pub update_light_config: bool,
}
