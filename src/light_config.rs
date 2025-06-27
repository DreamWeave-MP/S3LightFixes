use std::{
    fs::{File, read_dir, read_to_string},
    io::{self, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{DEFAULT_CONFIG_NAME, default, notification_box, to_io_error};

#[derive(Debug, Deserialize, Serialize)]
pub struct LightConfig {
    /// This parameter is DANGEROUS
    /// It's only meant to be used with vtastek's experimental shaders for openmw 0.47
    /// <https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148>
    #[serde(skip)]
    pub disable_interior_sun: bool,

    #[serde(default = "default::disable_flicker")]
    pub disable_flickering: bool,

    #[serde(default = "default::disable_pulse")]
    pub disable_pulse: bool,

    #[serde(default = "default::save_log")]
    pub save_log: bool,

    #[serde(default = "default::standard_hue")]
    pub standard_hue: f32,

    #[serde(default = "default::standard_saturation")]
    pub standard_saturation: f32,

    #[serde(default = "default::standard_value")]
    pub standard_value: f32,

    #[serde(default = "default::standard_radius")]
    pub standard_radius: f32,

    #[serde(default = "default::colored_hue")]
    pub colored_hue: f32,

    #[serde(default = "default::colored_saturation")]
    pub colored_saturation: f32,

    #[serde(default = "default::colored_value")]
    pub colored_value: f32,

    #[serde(default = "default::colored_radius")]
    pub colored_radius: f32,

    #[serde(default = "default::duration_mult")]
    pub duration_mult: f32,

    #[serde(default)]
    pub excluded_ids: Vec<String>,

    #[serde(default = "default::excluded_plugins")]
    pub excluded_plugins: Vec<String>,
}

/// Primarily exists to provide default implementations
/// for field values
impl LightConfig {
    fn find(root_path: &PathBuf) -> Result<PathBuf, io::Error> {
        read_dir(root_path)?
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.file_name().eq_ignore_ascii_case(DEFAULT_CONFIG_NAME))
            .map(|entry| entry.path())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Light config not found"))
    }

    /// Gives back the lightconfig adjacent to openmw.cfg when called
    /// use_classic dictates whether or not a fixed radius of 2.0 will be used on orange-y lights
    /// and whether or not to disable interior sunlight
    /// the latter field is not de/serializable and can only be used via the --classic argument
    pub fn get(
        light_args: &crate::LightArgs,
        openmw_config: &openmw_config::OpenMWConfiguration,
    ) -> Result<LightConfig, io::Error> {
        let mut write_config = false;

        let mut light_config: LightConfig = if let Ok(config_path) =
            Self::find(&openmw_config.user_config_path())
        {
            let config_contents = read_to_string(config_path)?;

            match toml::from_str(&config_contents) {
                Ok(config) => config,
                Err(_) => {
                    notification_box(
                        "Failed to read light config!",
                        "Lightconfig.toml couldn't be read. It will be replaced with a default copy.",
                        light_args.no_notifications,
                    );
                    write_config = true;
                    LightConfig::default()
                }
            }
        } else {
            write_config = true;
            LightConfig::default()
        };

        // Replace any values provided as CLI args in the config
        // use_classic will always override the standard_radius and disable_interior_sun
        [
            (&mut light_config.standard_hue, light_args.standard_hue),
            (
                &mut light_config.standard_saturation,
                light_args.standard_saturation,
            ),
            (&mut light_config.standard_value, light_args.standard_value),
            (
                &mut light_config.standard_radius,
                light_args.standard_radius,
            ),
            (&mut light_config.colored_hue, light_args.colored_hue),
            (
                &mut light_config.colored_saturation,
                light_args.colored_saturation,
            ),
            (&mut light_config.colored_value, light_args.colored_value),
            (&mut light_config.colored_radius, light_args.colored_radius),
            (&mut light_config.duration_mult, light_args.duration_mult),
        ]
        .iter_mut()
        .for_each(|(field, value)| {
            if let Some(v) = value {
                **field = std::mem::take(v);
            }
        });

        if let Some(status) = light_args.disable_flickering {
            light_config.disable_flickering = status
        }

        if let Some(status) = light_args.disable_pulse {
            light_config.disable_pulse = status
        }

        for id in &light_args.excluded_ids {
            light_config.excluded_ids.push(id.to_owned())
        }

        for id in &light_args.excluded_plugins {
            light_config.excluded_plugins.push(id.to_owned())
        }

        // This parameter indicates whether the user requested
        // To use compatibility mode for vtastek's old 0.47 shaders
        // via startup arguments
        // Drastically increases light radii
        // and disables interior sunlight
        if light_args.use_classic {
            light_config.standard_radius = 2.0;
            light_config.disable_interior_sun = true;
        }

        // If the configuration file didn't exist when we tried to find it,
        // serialize it here
        if write_config {
            let config_serialized =
                toml::to_string_pretty(&light_config).map_err(to_io_error)?;

            let config_path = openmw_config.user_config_path().join(DEFAULT_CONFIG_NAME);
            let mut config_file = File::create(config_path)?;
            write!(config_file, "{}", config_serialized)?;
        }

        Ok(light_config)
    }
}

impl Default for LightConfig {
    fn default() -> LightConfig {
        LightConfig {
            disable_interior_sun: false,
            disable_flickering: true,
            disable_pulse: false,
            save_log: false,
            standard_hue: default::standard_hue(),
            standard_saturation: default::standard_saturation(),
            standard_value: default::standard_value(),
            standard_radius: default::standard_radius(),
            colored_hue: default::colored_hue(),
            colored_saturation: default::colored_saturation(),
            colored_value: default::colored_value(),
            colored_radius: default::colored_radius(),
            duration_mult: default::duration_mult(),
            excluded_ids: Vec::new(),
            excluded_plugins: default::excluded_plugins(),
        }
    }
}
