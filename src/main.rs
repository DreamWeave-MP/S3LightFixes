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
use toml::{
    Table,
    Value::{Boolean, Float},
};

const CONFIG_SHOULD_ALWAYS_PARSE_ERR: &str =
    "Config was already loaded and should never fail to parse!";
const DEFAULT_AUTO_INSTALL: bool = true;
const DEFAULT_CONFIG_NAME: &str = "lightconfig.toml";
const DEFAULT_DISABLE_FLICKER: bool = true;
const DEFAULT_DO_LOG: bool = false;
const EMPTY_HEADER_ERR: &str = "The generated plugin was not found to have any master files! It's empty! Try running lightfixes again using the S3L_DEBUG environment variable";
const GET_CONFIG_ERR: &str = "Failed to read openmw.cfg from";
const GET_PLUGINS_ERR: &str = "Failed to read plugins in openmw.cfg from";
const LOG_NAME: &str = "lightconfig.log";
const NO_PLUGINS_ERR: &str = "No plugins were found in openmw.cfg! No lights to fix!";
const PLUGIN_LOAD_FAILED_ERR: &str = "Failed to load plugin from";
const PLUGIN_NAME: &str = "S3LightFixes.omwaddon";
const PLUGINS_MUST_EXIST_ERR: &str = "Plugins must exist to be loaded by openmw-cfg crate!";
const SHIT_GOT_REAL: &str = "Invalid value found when parsing light config!";
const STD_DEFAULT_HUE: f32 = 0.6;
const STD_DEFAULT_SAT: f32 = 0.8;
const STD_DEFAULT_VAL: f32 = 0.57;
const STD_DEFAULT_RAD: f32 = 2.0;
const STD_COLORED_HUE: f32 = 1.0;
const STD_COLORED_SAT: f32 = 0.9;
const STD_COLORED_VAL: f32 = 0.7;
const STD_COLORED_RAD: f32 = 1.1;

#[derive(Debug, Deserialize, Serialize)]
struct LightConfig {
    auto_install: bool,
    disable_flickering: bool,
    save_log: bool,
    standard_hue: f32,
    standard_saturation: f32,
    standard_value: f32,
    standard_radius: f32,
    colored_hue: f32,
    colored_saturation: f32,
    colored_value: f32,
    colored_radius: f32,
}

impl Default for LightConfig {
    fn default() -> LightConfig {
        LightConfig {
            auto_install: DEFAULT_AUTO_INSTALL,
            disable_flickering: DEFAULT_DISABLE_FLICKER,
            save_log: DEFAULT_DO_LOG,
            standard_hue: STD_DEFAULT_HUE,
            standard_saturation: STD_DEFAULT_SAT,
            standard_value: STD_DEFAULT_VAL,
            standard_radius: STD_DEFAULT_RAD,
            colored_hue: STD_COLORED_HUE,
            colored_saturation: STD_COLORED_SAT,
            colored_value: STD_COLORED_VAL,
            colored_radius: STD_COLORED_RAD,
        }
    }
}

fn openmw_config_path() -> String {
    let config_path = absolute_path_to_openmw_cfg();

    if config_path
        .to_string_lossy()
        .to_ascii_lowercase()
        .contains("openmw.cfg")
    {
        config_path
            .parent()
            .unwrap_or(&config_path)
            .to_string_lossy()
            .to_string()
    } else {
        config_path.to_string_lossy().to_string()
    }
}

fn find_light_config(data_local: &String) -> Option<String> {
    let directories = [
        std::env::current_exe()
            .ok()?
            .to_path_buf()
            .to_str()?
            .to_string(),
        openmw_config_path(),
        data_local.to_string(),
    ];

    for dir in directories.iter() {
        let file_path = std::path::Path::new(dir.as_str()).join(DEFAULT_CONFIG_NAME);
        if file_path.exists() {
            if let Ok(content) = read_to_string(&file_path) {
                return Some(content);
            }
        }
    }
    None
}

fn load_light_config(config_data: String) -> LightConfig {
    if let Ok(mut config_toml) = config_data.parse::<Table>() {
        LightConfig {
            auto_install: config_toml
                .remove("auto_install")
                .unwrap_or(Boolean(DEFAULT_AUTO_INSTALL))
                .as_bool()
                .expect(SHIT_GOT_REAL),
            disable_flickering: config_toml
                .remove("disable_flickering")
                .unwrap_or(Boolean(DEFAULT_DISABLE_FLICKER))
                .as_bool()
                .expect(SHIT_GOT_REAL),
            save_log: config_toml
                .remove("save_log")
                .unwrap_or(Boolean(DEFAULT_DO_LOG))
                .as_bool()
                .expect(SHIT_GOT_REAL),
            standard_hue: config_toml
                .remove("standard_hue")
                .unwrap_or(Float(STD_DEFAULT_HUE.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            standard_saturation: config_toml
                .remove("standard_desaturation")
                .unwrap_or(Float(STD_DEFAULT_SAT.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            standard_value: config_toml
                .remove("standard_value")
                .unwrap_or(Float(STD_DEFAULT_VAL.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            standard_radius: config_toml
                .remove("standard_radius")
                .unwrap_or(Float(STD_DEFAULT_RAD.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            colored_hue: config_toml
                .remove("colored_hue")
                .unwrap_or(Float(STD_COLORED_HUE.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            colored_saturation: config_toml
                .remove("colored_desaturation")
                .unwrap_or(Float(STD_COLORED_SAT.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            colored_value: config_toml
                .remove("colored_value")
                .unwrap_or(Float(STD_COLORED_VAL.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
            colored_radius: config_toml
                .remove("colored_radius")
                .unwrap_or(Float(STD_COLORED_RAD.into()))
                .as_float()
                .expect(SHIT_GOT_REAL) as f32,
        }
    } else {
        LightConfig::default()
    }
}

fn notification_box(title: &str, message: &str) {
    if var("S3L_NO_NOTIFICATIONS").is_err() {
        #[cfg(not(target_os = "android"))]
        let _ = native_dialog::MessageDialog::new()
            .set_title(title)
            .set_text(message)
            .show_alert();
        #[cfg(target_os = "android")]
        println!("{}", message);
    } else {
        println!("{}", message);
    }
}

fn main() -> Result<()> {
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

    let light_config = if let Some(light_config) = find_light_config(&userdata_dir) {
        load_light_config(light_config)
    } else {
        let openmw_config_dir = PathBuf::from(&absolute_path_to_openmw_cfg())
            .parent()
            .expect("Unable to get config parent directory!")
            .to_string_lossy()
            .to_string();

        let path: PathBuf = PathBuf::from(&openmw_config_dir).join(DEFAULT_CONFIG_NAME);

        let _ = write!(
            File::create(path).expect("Failed to create file"),
            "{}",
            toml_to_string(&LightConfig::default()).unwrap()
        );
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
