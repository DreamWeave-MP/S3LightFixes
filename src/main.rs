use std::{
    env::var,
    fs::{create_dir_all, read_to_string, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::exit,
};

use openmw_cfg::{
    config_path as absolute_path_to_openmw_cfg, get_config as get_openmw_cfg, get_data_local_dir,
    get_plugins,
};
use palette::{rgb::Srgb, FromColor, Hsv, IntoColor};
use serde::{Deserialize, Serialize};
use tes3::esp::*;

const CONFIG_SHOULD_ALWAYS_PARSE_ERR: &str =
    "Config was already loaded and should never fail to parse!";
const DEFAULT_CONFIG_NAME: &str = "lightconfig.toml";
const EMPTY_HEADER_ERR: &str = "The generated plugin was not found to have any master files! It's empty! Try running lightfixes again using the S3L_DEBUG environment variable";
const GET_CONFIG_ERR: &str = "Failed to read openmw.cfg from";
const GET_PLUGINS_ERR: &str = "Failed to read plugins in openmw.cfg from";
const LOG_NAME: &str = "lightconfig.log";
const NO_PLUGINS_ERR: &str = "No plugins were found in openmw.cfg! No lights to fix!";
const PLUGIN_LOAD_FAILED_ERR: &str = "Failed to load plugin from";
const PLUGIN_NAME: &str = "S3LightFixes.omwaddon";
const PLUGINS_MUST_EXIST_ERR: &str = "Plugins must exist to be loaded by openmw-cfg crate!";

#[derive(Debug, Deserialize, Serialize)]
struct LightConfig {
    auto_install: bool,
    disable_flickering: bool,
    save_log: bool,

    #[serde(default = "LightConfig::default_standard_hue")]
    standard_hue: f32,

    #[serde(default = "LightConfig::default_standard_saturation")]
    standard_saturation: f32,

    #[serde(default = "LightConfig::default_standard_value")]
    standard_value: f32,

    #[serde(default = "LightConfig::default_standard_radius")]
    standard_radius: f32,

    #[serde(default = "LightConfig::default_colored_hue")]
    colored_hue: f32,

    #[serde(default = "LightConfig::default_colored_saturation")]
    colored_saturation: f32,

    #[serde(default = "LightConfig::default_colored_value")]
    colored_value: f32,

    #[serde(default = "LightConfig::default_colored_radius")]
    colored_radius: f32,
}

/// Primarily exists to provide default implementations
/// for field values
impl LightConfig {
    fn default_standard_hue() -> f32 {
        0.6
    }

    fn default_standard_saturation() -> f32 {
        0.8
    }

    fn default_standard_value() -> f32 {
        0.57
    }

    /// Original default radius was 2.0
    /// But was only appropriate for vtastek shaders
    /// MOMW configs use 1.2
    fn default_standard_radius() -> f32 {
        1.2
    }

    fn default_colored_hue() -> f32 {
        1.0
    }

    fn default_colored_saturation() -> f32 {
        0.9
    }

    fn default_colored_value() -> f32 {
        0.7
    }

    fn default_colored_radius() -> f32 {
        1.1
    }
}

impl Default for LightConfig {
    fn default() -> LightConfig {
        LightConfig {
            auto_install: true,
            disable_flickering: true,
            save_log: false,
            standard_hue: Self::default_standard_hue(),
            standard_saturation: Self::default_standard_saturation(),
            standard_value: Self::default_standard_value(),
            standard_radius: Self::default_standard_radius(),
            colored_hue: Self::default_colored_hue(),
            colored_saturation: Self::default_colored_saturation(),
            colored_value: Self::default_colored_value(),
            colored_radius: Self::default_colored_radius(),
        }
    }
}

fn find_light_config() -> Result<PathBuf, io::Error> {
    std::fs::read_dir(openmw_cfg::config_path())?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.file_name().eq_ignore_ascii_case(DEFAULT_CONFIG_NAME))
        .map(|entry| entry.path())
        .ok_or_else(|| io::Error::new(std::io::ErrorKind::NotFound, "Light config not found"))
}

fn to_io_error<E: std::fmt::Display>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

/// Displays a notification taking title and message as argument
/// Should be behind an argument and not an env
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

fn main() -> io::Result<()> {
    let mut config = match get_openmw_cfg() {
        Ok(config) => config,
        Err(error) => {
            notification_box(
                &"Failed to read configuration file!",
                &format!("{} {:#?}!", GET_CONFIG_ERR, error),
            );
            exit(127);
        }
    };

    let plugins = match get_plugins(&config) {
        Ok(plugins) => plugins,
        Err(error) => {
            notification_box(
                &"Failed to read plugins from config!",
                &format!("{} {:#?}!", GET_PLUGINS_ERR, error),
            );
            exit(127);
        }
    };

    if var("S3L_DEBUG").is_ok() {
        dbg!(
            &openmw_cfg::config_path(),
            &config,
            &plugins,
            &openmw_cfg::get_data_dirs(&config)
        );
    }

    assert!(plugins.len() > 0, "{}", NO_PLUGINS_ERR);

    let userdata_dir = get_data_local_dir(&config);

    let light_config: LightConfig = if let Ok(config_path) = find_light_config() {
        let config_contents = std::fs::read_to_string(config_path)?;

        toml::from_str(&config_contents).map_err(to_io_error)?
    } else {
        LightConfig::default()
    };

    let mut generated_plugin = Plugin::new();
    let mut used_ids: Vec<String> = Vec::new();

    let mut header = Header {
        version: 1.3,
        author: FixedString("S3".to_string()),
        description: FixedString("Plugin generated by s3-lightfixes".to_string()),
        ..Default::default()
    };

    for plugin_path in plugins.iter().rev() {
        if plugin_path.to_string_lossy().contains(PLUGIN_NAME) {
            continue;
        }

        let extension = match plugin_path.extension() {
            Some(extension) => extension.to_ascii_lowercase(),
            None => continue,
        };

        // You really should only have elder scrolls plugins as content entries
        // but I simply do not trust these people
        if extension != "esp"
            && extension != "esm"
            && extension != "omwaddon"
            && extension != "omwgame"
        {
            continue;
        }

        let mut plugin = match Plugin::from_path(plugin_path) {
            Ok(plugin) => plugin,
            Err(e) => {
                eprintln!(
                    "{} {}: {}",
                    PLUGIN_LOAD_FAILED_ERR,
                    plugin_path.display(),
                    e
                );
                continue;
            }
        };
        let mut used_objects = 0;

        // Disable sunlight color for true interiors
        for cell in plugin.objects_of_type::<Cell>() {
            let cell_id = cell.editor_id_ascii_lowercase().to_string();

            if !cell.data.flags.contains(CellFlags::IS_INTERIOR) || used_ids.contains(&cell_id) {
                continue;
            };

            let mut cell_copy = cell.clone();
            cell_copy.references.clear();
            if let Some(atmosphere) = &cell_copy.atmosphere_data {
                cell_copy.atmosphere_data = Some(AtmosphereData {
                    sunlight_color: [0, 0, 0, 0],
                    fog_density: atmosphere.fog_density,
                    fog_color: atmosphere.fog_color,
                    ambient_color: atmosphere.ambient_color,
                })
            }

            generated_plugin.objects.push(TES3Object::Cell(cell_copy));
            used_ids.push(cell_id);
            used_objects += 1;
        }

        for light in plugin.objects_of_type_mut::<Light>() {
            let light_id = light.editor_id_ascii_lowercase().to_string();
            if used_ids.contains(&light_id) {
                continue;
            }

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
                let rgb = Srgb::new(
                    light.data.color[0],
                    light.data.color[1],
                    light.data.color[2],
                )
                .into_format();

                let mut hsv: Hsv = Hsv::from_color(rgb);
                let hue = hsv.hue.into_degrees();

                if hue > 64.0 || hue < 14.0 {
                    light.data.radius =
                        (light_config.colored_radius * light.data.radius as f32) as u32;
                    hsv = Hsv::new(
                        hue * light_config.colored_hue,
                        hsv.saturation * light_config.colored_saturation,
                        hsv.value * light_config.colored_value,
                    );
                } else {
                    light.data.radius =
                        (light_config.standard_radius * light.data.radius as f32) as u32;
                    hsv = Hsv::new(
                        hue * light_config.standard_hue,
                        hsv.saturation * light_config.standard_saturation,
                        hsv.value * light_config.standard_value,
                    );
                }

                let rgbf_color: Srgb = hsv.into_color();
                let rgb8_color: Srgb<u8> = rgbf_color.into_format();

                light.data.color = [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0];
            }

            generated_plugin
                .objects
                .push(TES3Object::Light(light.clone()));
            used_ids.push(light_id);
            used_objects += 1;
        }

        if used_objects > 0 {
            let plugin_size = std::fs::metadata(plugin_path)?.len();
            let plugin_string = plugin_path
                .file_name()
                .expect(PLUGINS_MUST_EXIST_ERR)
                .to_string_lossy()
                .to_owned()
                .to_string();
            header.masters.insert(0, (plugin_string, plugin_size))
        }
    }

    if var("S3L_DEBUG").is_ok() {
        dbg!(&header);
    }

    assert!(header.masters.len() > 0, "{}", EMPTY_HEADER_ERR);

    generated_plugin.objects.push(TES3Object::Header(header));
    generated_plugin.sort_objects();

    let userdata_path = Path::new(&userdata_dir);
    if !userdata_path.exists() {
        create_dir_all(userdata_path)?;
    }

    let plugin_path = Path::new(&userdata_dir).join(PLUGIN_NAME);
    let _ = generated_plugin.save_path(plugin_path);

    if light_config.auto_install {
        let has_lightfixes_iter = config
            .section_mut::<String>(None)
            .expect(CONFIG_SHOULD_ALWAYS_PARSE_ERR)
            .get_all("content")
            .find(|s| *s == PLUGIN_NAME);

        if let None = has_lightfixes_iter {
            let config_path = absolute_path_to_openmw_cfg();

            let config_string = read_to_string(&config_path)?;
            if !config_string.contains(PLUGIN_NAME) {
                let mut file = OpenOptions::new().append(true).open(&config_path)?;
                if !config_string.ends_with('\n') {
                    write!(file, "{}", '\n')?
                }
                writeln!(file, "{}", format!("content={}\n", PLUGIN_NAME))?;
            }
        }
    }

    if light_config.save_log {
        let config_path = absolute_path_to_openmw_cfg()
            .parent()
            .expect("Unable to get config path parent!")
            .to_string_lossy()
            .to_string();

        let path: PathBuf = Path::new(&config_path).join(LOG_NAME);
        let mut file = File::create(path)?;
        let _ = write!(file, "{}", format!("{:#?}", &generated_plugin));
    }

    notification_box(
        &"Lightfixes successful!",
        &format!(
            "S3LightFixes.omwaddon generated, enabled, and saved in {}",
            openmw_cfg::get_data_local_dir(&config)
        ),
    );

    Ok(())
}
