use std::{
    env::{self, current_dir, var},
    fs::{create_dir_all, metadata, read_dir, read_to_string, remove_file, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::exit,
};

use clap::Parser;
use openmw_cfg::{
    config_path as absolute_path_to_openmw_cfg, get_config as get_openmw_cfg, get_data_local_dir,
    get_plugins,
};
use palette::{rgb::Srgb, FromColor, Hsv, IntoColor};
use serde::{Deserialize, Serialize};
use tes3::esp::*;

const DEFAULT_CONFIG_NAME: &str = "lightconfig.toml";
const LOG_NAME: &str = "lightconfig.log";
const PLUGIN_NAME: &str = "S3LightFixes.omwaddon";

#[derive(Parser, Debug)]
#[command(
    name = "S3 Lightfixes",
    about = "A tool for modifying light values globally across an OpenMW installation."
)]
struct LightArgs {
    /// Path to openmw.cfg
    /// By default, uses the system paths defined by:
    ///
    /// https://openmw.readthedocs.io/en/latest/reference/modding/paths.html
    ///
    /// Alternatively, responds to both the `OPENMW_CONFIG` and `OPENMW_CONFIG_DIR`
    /// environment variables.
    #[arg(short = 'c', long = "openmw-cfg")]
    openmw_cfg: Option<PathBuf>,

    /// Enables classic mode using vtastek shaders.
    /// ONLY for openmw 0.47. Relevant shaders can be found in the OpenMW discord:
    ///
    /// https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148
    #[arg(short = '7', long = "classic")]
    use_classic: bool,

    /// Output file path.
    /// Accepts relative and absolute terms.
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Whether to save a text form of the generated plugin.
    /// Extremely verbose!
    ///
    /// You probably don't want to enable this unless asked specifically to do so.
    #[arg(short = 'l', long = "write-log")]
    write_log: bool,

    /// Whether to automatically enable the output plugin in openmw.cfg.
    /// Disabled by default, and only available via CLI.
    ///
    /// Typically lightfixes is ran under momw-configurator, making this param
    /// unnecessary for many users.
    #[arg(short = 'e', long = "auto-enable")]
    auto_enable: bool,

    /// Whether to use notifications at runtime
    /// NOT available on android
    #[arg(short = 'n', long = "no-notifications")]
    no_notifications: bool,

    /// Output debugging information during lightfixes generation
    /// Primarily displays output related to the openmw.cfg being used for generation
    #[arg(short = 'd', long = "debug")]
    debug: bool,

    /// Outputs version info
    // Might be more later?
    #[arg(short = 'i', long = "info")]
    info: bool,
}

mod default {
    pub fn standard_hue() -> f32 {
        0.62
    }

    pub fn standard_saturation() -> f32 {
        0.8
    }

    pub fn standard_value() -> f32 {
        0.57
    }

    /// Original default radius was 2.0
    /// But was only appropriate for vtastek shaders
    /// MOMW configs use 1.2
    pub fn standard_radius() -> f32 {
        1.2
    }

    pub fn colored_hue() -> f32 {
        1.0
    }

    pub fn colored_saturation() -> f32 {
        0.9
    }

    pub fn colored_value() -> f32 {
        0.7
    }

    pub fn colored_radius() -> f32 {
        1.1
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LightConfig {
    /// This parameter is DANGEROUS
    /// It's only meant to be used with vtastek's experimental shaders for openmw 0.47
    /// <https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148>
    #[serde(skip)]
    disable_interior_sun: bool,
    disable_flickering: bool,
    save_log: bool,

    #[serde(default = "default::standard_hue")]
    standard_hue: f32,

    #[serde(default = "default::standard_saturation")]
    standard_saturation: f32,

    #[serde(default = "default::standard_value")]
    standard_value: f32,

    #[serde(default = "default::standard_radius")]
    standard_radius: f32,

    #[serde(default = "default::colored_hue")]
    colored_hue: f32,

    #[serde(default = "default::colored_saturation")]
    colored_saturation: f32,

    #[serde(default = "default::colored_value")]
    colored_value: f32,

    #[serde(default = "default::colored_radius")]
    colored_radius: f32,
}

/// Primarily exists to provide default implementations
/// for field values
impl LightConfig {
    fn find() -> Result<PathBuf, io::Error> {
        read_dir(absolute_path_to_openmw_cfg())?
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.file_name().eq_ignore_ascii_case(DEFAULT_CONFIG_NAME))
            .map(|entry| entry.path())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Light config not found"))
    }

    /// Gives back the lightconfig adjacent to openmw.cfg when called
    /// use_classic dictates whether or not a fixed radius of 2.0 will be used on orange-y lights
    /// and whether or not to disable interior sunlight
    /// the latter field is not de/serializable and can only be used via the --classic argument
    pub fn get(use_classic: bool) -> Result<LightConfig, io::Error> {
        let mut light_config: LightConfig = if let Ok(config_path) = Self::find() {
            let config_contents = read_to_string(config_path)?;

            toml::from_str(&config_contents).map_err(to_io_error)?
        } else {
            LightConfig::default()
        };

        // This parameter indicates whether the user requested
        // To use compatibility mode for vtastek's old 0.47 shaders
        // via startup arguments
        // Drastically increases light radii
        // and disables interior sunlight
        if use_classic {
            light_config.standard_radius = 2.0;
            light_config.disable_interior_sun = true;
        }

        Ok(light_config)
    }
}

impl Default for LightConfig {
    fn default() -> LightConfig {
        LightConfig {
            disable_interior_sun: false,
            disable_flickering: true,
            save_log: false,
            standard_hue: default::standard_hue(),
            standard_saturation: default::standard_saturation(),
            standard_value: default::standard_value(),
            standard_radius: default::standard_radius(),
            colored_hue: default::colored_hue(),
            colored_saturation: default::colored_saturation(),
            colored_value: default::colored_value(),
            colored_radius: default::colored_radius(),
        }
    }
}

fn to_io_error<E: std::fmt::Display>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

/// Displays a notification taking title and message as argument
fn notification_box(title: &str, message: &str) {
    #[cfg(target_os = "android")]
    println!("{}", message);

    #[cfg(not(target_os = "android"))]
    if var("S3L_NO_NOTIFICATIONS").is_err() {
        let _ = native_dialog::MessageDialog::new()
            .set_title(title)
            .set_text(message)
            .show_alert();
    } else {
        println!("{}", message);
    }
}

fn is_fixable_plugin(plug_path: &PathBuf) -> bool {
    // If path doesn't exist
    if std::fs::metadata(plug_path).is_err() {
        return false;
    // If path is the lightfixes plugin
    } else if plug_path.to_string_lossy().contains(PLUGIN_NAME) {
        return false;
    } else {
        // Don't match extensionless files
        // And also do the match in case-insensitive fashion
        match plug_path.extension() {
            None => return false,
            Some(ext) => match ext.to_ascii_lowercase().to_str().unwrap_or_default() {
                "esp" | "esm" | "omwaddon" | "omwgame" => return true,
                _ => return false,
            },
        }
    }
}

fn save_plugin(output_dir: &PathBuf, generated_plugin: &mut Plugin) -> io::Result<()> {
    let mut plugin_path = output_dir.join(PLUGIN_NAME);

    match metadata(output_dir) {
        Ok(metadata) if !metadata.is_dir() => {
            let cwd =
                current_dir().expect("CRITICAL FAILURE: FAILED TO READ CURRENT WORKING DIRECTORY!");

            eprintln!("WARNING: Couldn't use {} as an output directory, as it isn't a directory. Using the current working directory, {}, instead!", output_dir.display(), cwd.display());

            plugin_path = cwd.join(PLUGIN_NAME);
        }
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            create_dir_all(output_dir)?;
        }
        Err(err) => return Err(err),
    }

    generated_plugin.save_path(plugin_path)?;

    Ok(())
}

fn validate_config_dir(dir: &PathBuf) -> io::Result<()> {
    let dir_metadata = metadata(&dir);
    let default_location = absolute_path_to_openmw_cfg();

    let config_arg_fail = match dir_metadata.is_ok() && dir_metadata.unwrap().is_dir() {
            false => Some(format!("The requested openmw.cfg dir {} is not a directory! Using the system default location of {} instead.",
                dir.display(),
               &default_location.display())),
            true => {
                match read_dir(&dir)?
                .filter_map(|entry| entry.ok())
                .find(|entry| entry.file_name().eq_ignore_ascii_case("openmw.cfg"))
                .map(|entry| entry.path()) {
                    None => Some(format!("An openmw.cfg could not be located in {}! Lightfixes will use the system default location of {} instead.", dir.display(), &default_location.display())),
                    Some(dir) => {
                        env::set_var("OPENMW_CONFIG", &dir);
                        None
                    }
                }
            }

        };

    if let Some(fail_message) = config_arg_fail {
        eprintln!("{}", fail_message);
    };

    Ok(())
}

fn main() -> io::Result<()> {
    let args = LightArgs::parse();

    if args.info {
        println!("S3LightFixes Version: {}", env!("CARGO_PKG_VERSION"),);
        exit(0);
    };

    let output_dir = match args.output {
        Some(dir) => dir,
        None => current_dir().expect("CRITICAL FAILURE: FAILED TO READ CURRENT WORKING DIRECTORY!"),
    };

    // If the openmw.cfg path is provided by the user, force the crate to use
    // whatever they've provided
    if let Some(dir) = args.openmw_cfg {
        if let Err(error) = validate_config_dir(&dir) {
            eprintln!(
                "WARNING: Failed setting config directory to {} due to : {} . Lightfixes will use the default config directory of {} instead.",
                &dir.display(),
                error.to_string(),
                absolute_path_to_openmw_cfg().display(),
            )
        };
    }

    let mut config = match get_openmw_cfg() {
        Ok(config) => config,
        Err(error) => {
            let config_fail = format!("{} {:#?}!", "Failed to read openmw.cfg from", error);

            if !args.no_notifications {
                notification_box(&"Failed to read configuration file!", &config_fail);
            } else {
                eprintln!("{}", config_fail);
            }

            exit(127);
        }
    };

    let plugins = match get_plugins(&config) {
        Ok(plugins) => plugins,
        Err(error) => {
            let plugin_fail = &format!(
                "{} {:#?}!",
                "Failed to read plugins in openmw.cfg from", error
            );

            if !args.no_notifications {
                notification_box(&"Failed to read plugins from config!", plugin_fail);
            } else {
                eprintln!("{}", plugin_fail);
            }

            exit(127);
        }
    };

    if var("S3L_DEBUG").is_ok() || args.debug {
        dbg!(&openmw_cfg::config_path(), &config);
    }

    assert!(
        plugins.len() > 0,
        "No plugins were found in openmw.cfg! No lights to fix!"
    );

    let light_config = LightConfig::get(args.use_classic)?;

    let mut generated_plugin = Plugin::new();
    let mut used_ids: Vec<String> = Vec::new();

    let mut header = Header {
        version: 1.3,
        author: FixedString("S3".to_string()),
        description: FixedString("Plugin generated by s3-lightfixes".to_string()),
        file_type: types::FileType::Esp,
        flags: ObjectFlags::default(),
        num_objects: 0,
        masters: Vec::new(),
    };

    let mut used_objects = 0;
    for plugin_path in plugins.iter().rev() {
        if !is_fixable_plugin(&plugin_path) {
            continue;
        };

        let mut plugin = match Plugin::from_path(plugin_path) {
            Ok(plugin) => plugin,
            Err(e) => {
                eprintln!(
                    "WARNING: Plugin {}: could not be loaded due to error: {}. Continuing light fixes without this mod . . .",
                    plugin_path.display(),
                    e
                );
                continue;
            }
        };

        // Disable sunlight color for true interiors
        // Only do this for `classic` mode
        if light_config.disable_interior_sun {
            for cell in plugin.objects_of_type_mut::<Cell>() {
                let cell_id = cell.editor_id_ascii_lowercase().to_string();

                if !cell.data.flags.contains(CellFlags::IS_INTERIOR) || used_ids.contains(&cell_id)
                {
                    continue;
                };

                cell.references.clear();
                // Take the cell for ourselves instead of cloning it
                let mut owned_cell = std::mem::take(cell);

                if let Some(mut atmosphere) = owned_cell.atmosphere_data.take() {
                    atmosphere.sunlight_color = [0, 0, 0, 0];
                    owned_cell.atmosphere_data = Some(atmosphere);
                }

                generated_plugin.objects.push(TES3Object::Cell(owned_cell));
                used_ids.push(cell_id);
                used_objects += 1;
            }
        }

        for light in plugin.objects_of_type_mut::<Light>() {
            let light_id = light.editor_id_ascii_lowercase().to_string();
            if used_ids.contains(&light_id) {
                continue;
            }

            let mut light = std::mem::take(light);

            if light_config.disable_flickering {
                light
                    .data
                    .flags
                    .remove(LightFlags::FLICKER | LightFlags::FLICKER_SLOW);
            }

            if light.data.flags.contains(LightFlags::NEGATIVE) {
                light.data.flags.remove(LightFlags::NEGATIVE);
                light.data.radius = 0;
            } else {
                let light_as_rgb = Srgb::new(
                    light.data.color[0],
                    light.data.color[1],
                    light.data.color[2],
                )
                .into_format();

                let mut light_as_hsv: Hsv = Hsv::from_color(light_as_rgb);
                let light_hue = light_as_hsv.hue.into_degrees();

                let (radius, hue, saturation, value) = match light_hue > 64. || light_hue < 14. {
                    // Red, purple, blue, green, yellow
                    true => (
                        light_config.colored_radius,
                        light_config.colored_hue,
                        light_config.colored_saturation,
                        light_config.colored_value,
                    ),
                    // Everything else
                    false => (
                        light_config.standard_radius,
                        light_config.standard_hue,
                        light_config.standard_saturation,
                        light_config.standard_value,
                    ),
                };

                light.data.radius = (radius * light.data.radius as f32) as u32;
                light_as_hsv = Hsv::new(
                    light_hue * hue,
                    light_as_hsv.saturation * saturation,
                    light_as_hsv.value * value,
                );

                let rgbf_color: Srgb = light_as_hsv.into_color();
                let rgb8_color: Srgb<u8> = rgbf_color.into_format();

                light.data.color = [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0];
            }

            generated_plugin.objects.push(TES3Object::Light(light));
            used_ids.push(light_id);
            used_objects += 1;
        }

        if used_objects > 0 {
            let plugin_size = metadata(plugin_path)?.len();
            let plugin_string = plugin_path
                .file_name()
                .expect("Plugins must exist to be loaded by openmw-cfg crate!")
                .to_string_lossy()
                .to_string();
            header.masters.insert(0, (plugin_string, plugin_size));

            used_objects = 0;
        }
    }

    if var("S3L_DEBUG").is_ok() {
        dbg!(&header);
    }

    assert!(header.masters.len() > 0, "The generated plugin was not found to have any master files! It's empty! Try running lightfixes again using the S3L_DEBUG environment variable");

    header.num_objects = used_objects;
    generated_plugin.objects.push(TES3Object::Header(header));
    generated_plugin.sort_objects();

    let openmw_config_path = absolute_path_to_openmw_cfg();
    let openmw_config_dir = openmw_config_path
        .parent()
        .expect("Unable to read parent directory of openmw.cfg!");

    // If the old plugin format exists, remove it
    // Do it before serializing the new plugin, as the target dir may still be the old one
    let legacy_path = Path::new(&get_data_local_dir(&config)).join(PLUGIN_NAME);
    if metadata(&legacy_path).is_ok() {
        remove_file(legacy_path)?;
    };

    save_plugin(&output_dir, &mut generated_plugin)?;

    // Handle this arg via clap
    if args.auto_enable {
        let has_lightfixes_iter = config
            .section_mut::<String>(None)
            .expect("CRITICAL ERROR: CONFIG WAS ALREADY LOADED AND SHOULD NEVER FAIL PARSING!")
            .get_all("content")
            .find(|s| *s == PLUGIN_NAME);

        if let None = has_lightfixes_iter {
            let config_string = read_to_string(&openmw_config_path)?;
            if !config_string.contains(PLUGIN_NAME) {
                let mut file = OpenOptions::new().append(true).open(&openmw_config_path)?;
                if !config_string.ends_with('\n') {
                    write!(file, "{}", '\n')?
                }
                writeln!(file, "{}", format!("content={}\n", PLUGIN_NAME))?;
            }
        }
    }

    if args.write_log || light_config.save_log {
        let path = openmw_config_dir.join(LOG_NAME);
        let mut file = File::create(path)?;
        let _ = write!(file, "{}", format!("{:#?}", &generated_plugin));
    }

    let lights_fixed = format!(
        "S3LightFixes.omwaddon generated, enabled, and saved in {}",
        output_dir.display()
    );

    if !args.no_notifications {
        notification_box(&"Lightfixes successful!", &lights_fixed);
    } else {
        println!("{}", lights_fixed);
    };

    Ok(())
}
