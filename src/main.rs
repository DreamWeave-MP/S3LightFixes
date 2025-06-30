use std::{
    collections::HashSet,
    env::{current_dir, var},
    fs::{File, metadata, remove_file},
    io::{self, Write},
    mem::take as TakeAndSwitch,
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use palette::{FromColor, GetHue, Hsv, IntoColor, rgb::Srgb};
use rayon::prelude::*;
use tes3::esp::{
    Cell, CellFlags, EditorId, FixedString, Header, Light, LightFlags, ObjectFlags, Plugin,
    TES3Object, types::FileType,
};
use vfstool_lib::VFS;

use s3lightfixes::{
    LOG_NAME, LightArgs, LightConfig, PLUGIN_NAME, get_config_path, is_fixable_plugin,
    notification_box, save_plugin,
};

/// Given a LightData reference from an ESP light,
/// returns the HSV version and whether it is colored or not (for the global modifier)
pub fn light_to_hsv(light_data: &tes3::esp::LightData) -> (Hsv, bool) {
    let rgb: palette::rgb::Rgb = Srgb::new(
        light_data.color[0],
        light_data.color[1],
        light_data.color[2],
    )
    .into_format();

    let hsv: Hsv = Hsv::from_color(rgb);
    let hue_degrees = hsv.get_hue().into_positive_degrees();

    (hsv, hue_degrees > 64. || hue_degrees < 14.)
}

pub fn process_light(light_config: &LightConfig, light: &mut tes3::esp::Light) {
    if light_config.disable_flickering {
        light
            .data
            .flags
            .remove(LightFlags::FLICKER | LightFlags::FLICKER_SLOW);
    }

    if light_config.disable_pulse {
        light
            .data
            .flags
            .remove(LightFlags::PULSE | LightFlags::PULSE_SLOW);
    }

    if light.data.flags.contains(LightFlags::NEGATIVE) {
        light.data.flags.remove(LightFlags::NEGATIVE);
        light.data.radius = 0;
        light.data.color = [0, 0, 0, 0];
        return;
    }

    let (mut light_as_hsv, is_colored) = light_to_hsv(&light.data);

    let (radius, hue, saturation, value) = match is_colored {
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

    light_as_hsv = Hsv::new(
        palette::RgbHue::from_degrees((light_as_hsv.hue.into_raw_degrees() * hue).clamp(0., 360.)),
        light_as_hsv.saturation * saturation,
        light_as_hsv.value * value,
    );

    let rgb8_color: Srgb<u8> = <Hsv as IntoColor<Srgb>>::into_color(light_as_hsv).into_format();

    light.data.radius = (radius * light.data.radius as f32) as u32;
    light.data.color = [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0];
    light.data.time = (light.data.time as f32 * light_config.duration_mult) as i32;
}

fn main() -> io::Result<()> {
    let mut args = LightArgs::parse();

    if args.info {
        println!("S3LightFixes Version: {}", env!("CARGO_PKG_VERSION"),);
        exit(0);
    };

    let no_notifications = var("S3L_NO_NOTIFICATIONS").is_ok() || args.no_notifications;
    let config_dir = get_config_path(&mut args);

    // If the openmw.cfg path is provided by the user, force the crate to use
    // whatever they've provided
    let mut config = match openmw_config::OpenMWConfiguration::new(Some(config_dir)) {
        Ok(config) => config,
        Err(error) => {
            notification_box(
                &"Failed to read configuration file!",
                &error.to_string(),
                no_notifications,
            );

            exit(127);
        }
    };

    let output_dir = match args.output {
        Some(ref dir) => {
            if dir.is_dir() {
                dir.to_owned()
            } else {
                notification_box(
                    "Can't find output location!",
                    "WARNING: The requested output path {dir:?} does not exist! Terminating.",
                    no_notifications,
                );
                exit(1)
            }
        }

        None => match &mut config.data_local() {
            Some(dir) => dir.parsed().to_owned(),
            None => match current_dir() {
                Ok(dir) => dir,
                Err(_) => {
                    notification_box(
                        "Can't get workdir!",
                        "[ CRITICAL FAILURE ]: FAILED TO READ CURRENT WORKING DIRECTORY!",
                        no_notifications,
                    );
                    std::process::exit(256);
                }
            },
        },
    };

    let light_config = LightConfig::get(args, &config)?;

    if light_config.debug {
        dbg!(&light_config, &config);
    }

    if config.content_files().len() == 0 {
        notification_box(
            "No Plugins!",
            "No plugins were found in openmw.cfg! No lights to fix!",
            light_config.no_notifications,
        );
        std::process::exit(4);
    }

    let mut generated_plugin = Plugin::new();
    let mut used_ids: HashSet<String> = HashSet::new();

    let mut header = Header {
        version: 1.3,
        author: FixedString("S3".to_string()),
        description: FixedString("Plugin generated by s3-lightfixes".to_string()),
        file_type: FileType::Esp,
        flags: ObjectFlags::default(),
        num_objects: 0,
        masters: Vec::new(),
    };

    let directories: Vec<&PathBuf> = config.data_directories();

    let vfs = VFS::from_directories(directories, None);

    let plugins = config
    .content_files()
    .par_iter()
    .rev()
    .filter_map(|plugin| {
        let vfs_file = vfs.get_file(plugin)?;
        let path = vfs_file.path();

        if !is_fixable_plugin(path) {
            return None;
        }

        match Plugin::from_path_filtered(path, |tag| matches!(&tag, Cell::TAG | Light::TAG)) {
            Ok(plugin) => Some((plugin, path)),
            Err(err) => {
                eprintln!(
                    "[ WARNING ]: Plugin {}: could not be loaded due to error: {}. Continuing light fixes without this mod .  . . Everything will be okay. Yes, it's still working.\n",
                    path.display(),
                    err
                );
                None
            }
        }
    })
    .collect::<Vec<_>>();

    let mut used_objects = 0;
    for (mut plugin, plugin_path) in plugins {
        // Disable sunlight color for true interiors
        // Only do this for `classic` mode
        if light_config.disable_interior_sun {
            for cell in plugin.objects_of_type_mut::<Cell>().filter(|cell| {
                cell.data.flags.contains(CellFlags::IS_INTERIOR) && cell.atmosphere_data.is_some()
            }) {
                let cell_id = cell.editor_id_ascii_lowercase().into_owned();

                if used_ids.contains(&cell_id) {
                    continue;
                };

                match cell.atmosphere_data {
                    Some(ref mut atmo) => {
                        cell.references.clear();

                        atmo.sunlight_color = [0, 0, 0, 0];

                        generated_plugin.objects.push(TakeAndSwitch(cell).into());

                        used_ids.insert(cell_id);
                        used_objects += 1;
                    }
                    None => {}
                }
            }
        }

        plugin
            .into_objects_of_type::<Light>()
            .filter_map(|light| {
                let light_id = light.editor_id_ascii_lowercase().into_owned();

                if !used_ids.contains(&light_id) && !light_config.is_excluded_id(&light_id) {
                    used_ids.insert(light_id);
                    Some(light)
                } else {
                    None
                }
            })
            .for_each(|mut light| {
                process_light(&light_config, &mut light);

                generated_plugin.objects.push(light.into());
                used_objects += 1;
            });

        if used_objects > 0 {
            let plugin_size = metadata(plugin_path)?.len();
            let plugin_string = match plugin_path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => {
                    notification_box(
                        "Bad plugin path!",
                        "Lightfixes could not resolve the name of one of your plugins! This is UBER Bad and should never happen!",
                        light_config.no_notifications,
                    );
                    std::process::exit(3);
                }
            };

            header.masters.insert(0, (plugin_string, plugin_size));

            header.num_objects += TakeAndSwitch(&mut used_objects);
        }
    }

    if light_config.debug {
        dbg!(&header);
    }

    if header.masters.len() == 0 {
        notification_box(
            "No masters found!",
            "The generated plugin was not found to have any master files! It's empty! Try running lightfixes again using the S3L_DEBUG environment variable",
            light_config.no_notifications,
        );
        std::process::exit(2);
    }

    generated_plugin.objects.push(TES3Object::Header(header));
    generated_plugin.sort_objects();

    // If the old plugin format exists, remove it
    // Do it before serializing the new plugin, as the target dir may still be the old one
    if let Some(dir) = &mut config.data_local() {
        let old_plug_path = dir.parsed().join(PLUGIN_NAME);
        if old_plug_path.is_file() {
            let _ = remove_file(old_plug_path);
        }
    }

    if let Err(err) = save_plugin(&output_dir, &mut generated_plugin) {
        notification_box(
            "Failed to save plugin!",
            &err.to_string(),
            light_config.no_notifications,
        );
    };

    // Handle this arg via clap
    if light_config.auto_enable {
        if !config.has_content_file(&PLUGIN_NAME) {
            match config.add_content_file(&PLUGIN_NAME) {
                Ok(_) => {
                    if let Err(err) = config.save_user() {
                        notification_box(
                            "Failed to resave openmw.cfg!",
                            &err,
                            light_config.no_notifications,
                        );
                    } else {
                        let lightfix_enabled_msg = format!(
                            "Wrote user openmw.cfg at {} successfully!",
                            config.user_config_path().display()
                        );
                        notification_box(
                            "Lightfixes enabled!",
                            &lightfix_enabled_msg,
                            light_config.no_notifications,
                        );
                    }
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(256);
                }
            };
        }
    }

    if light_config.save_log {
        let path = config.user_config_path().join(LOG_NAME);
        let mut file = File::create(path)?;
        let _ = write!(file, "{}", format!("{:#?}", &generated_plugin));
    }

    let lights_fixed = format!(
        "S3LightFixes.omwaddon generated, enabled, and saved in {}",
        output_dir.display()
    );

    notification_box(
        &"Lightfixes successful!",
        &lights_fixed,
        light_config.no_notifications,
    );

    Ok(())
}
