use std::path::PathBuf;

use clap::Parser;

use crate::default;

#[derive(Parser, Debug)]
#[command(
    name = "S3 Lightfixes",
    about = "A tool for modifying light values globally across an OpenMW installation.\nPlease note that arguments provided here, which also exist in lightConfig.toml, will override any values in lightConfig.toml when used.\nAdditionally, if the lightConfig.toml does not exist, the used values will be saved into the new lightConfig.toml."
)]
pub struct LightArgs {
    /// Path to openmw.cfg
    /// By default, uses the system paths defined by:
    /// https://openmw.readthedocs.io/en/latest/reference/modding/paths.html
    /// Can be the literal path to an openmw.cfg file (including not literally being called openmw.cfg)
    /// Or the directory in which an openmw.cfg file lives.
    #[arg(short = 'c', long = "openmw-cfg")]
    pub openmw_cfg: Option<PathBuf>,

    /// Enables classic mode using vtastek shaders.
    /// ONLY for openmw 0.47. Relevant shaders can be found in the OpenMW discord:
    /// https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148
    #[arg(short = '7', long = "classic")]
    pub use_classic: bool,

    /// Output file path.
    /// Accepts relative and absolute terms.
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Whether to save a text form of the generated plugin.
    /// Extremely verbose!
    /// You probably don't want to enable this unless asked specifically to do so.
    #[arg(short = 'l', long = "write-log")]
    pub write_log: bool,

    /// Whether to automatically enable the output plugin in openmw.cfg.
    /// Disabled by default, and only available via CLI.
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

    /// Outputs version info
    // Might be more later?
    #[arg(short = 'i', long = "info")]
    pub info: bool,

    /// Whether to disable flickering lights during lightfixes generation
    #[arg(short = 'f', long = "no-flicker")]
    pub disable_flickering: Option<bool>,

    /// Whether to disable pulsing lights during lightfixes generation
    #[arg(short = 'p', long = "no-pulse")]
    pub disable_pulse: Option<bool>,

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
        short = 'V',
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
        long = "excluded-ids",
        help = &format!("List of Regex patterns of light recordIds to exclude. This setting is *merged* onto values defined by lightconfig.toml.\nIf this argument is not used, the value will be derived from lightConfig.toml.")
    )]
    pub excluded_ids: Vec<String>,

    #[arg(
        long = "excluded-plugins",
        help = &format!("List of Regex patterns of plugins to exclude. This setting is *merged* onto values defined by lightconfig.toml.\nIf this argument is not used, the value will be derived from lightConfig.toml.")
    )]
    pub excluded_plugins: Vec<String>,
}