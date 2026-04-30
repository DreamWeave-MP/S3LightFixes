use std::{
    collections::HashSet,
    env::{current_dir, var},
    fs::{File, metadata, remove_file},
    io::{self, Write},
    mem::take,
    path::{Path, PathBuf},
    process::exit,
};

use clap::Parser;
use palette::{Hsv, IntoColor, rgb::Srgb};
use rayon::prelude::*;
use tes3::esp::{
    AtmosphereData, Cell, CellFlags, EditorId, FixedString, Header, Light, ObjectFlags, Plugin,
    TES3Object, types::FileType,
};
use vfstool_lib::VFS;

use crate::{
    LOG_NAME, LightArgs, LightConfig, PLUGIN_NAME, get_config_path, is_fixable_plugin,
    notification_box, save_plugin,
};

use crate::light_processing::{hue_degrees, process_light};

type LoadedPlugin<'a> = (Plugin, &'a Path);

struct GenerationResult {
    plugin: Plugin,
    header: Header,
}

fn load_openmw_config(
    args: &mut LightArgs,
    no_notifications: bool,
) -> openmw_config::OpenMWConfiguration {
    let config_dir = get_config_path(args);

    match openmw_config::OpenMWConfiguration::new(Some(config_dir)) {
        Ok(config) => config,
        Err(error) => {
            notification_box(
                "Failed to read configuration file!",
                &error.to_string(),
                no_notifications,
            );

            exit(127);
        }
    }
}

fn output_dir_from_args_or_config(
    args: &LightArgs,
    config: &mut openmw_config::OpenMWConfiguration,
    no_notifications: bool,
) -> PathBuf {
    if let Some(ref dir) = args.output {
        if dir.is_dir() {
            return dir.to_owned();
        }

        notification_box(
            "Can't find output location!",
            &format!(
                "WARNING: The requested output path {} does not exist! Terminating.",
                dir.display()
            ),
            no_notifications,
        );
        exit(1);
    }

    match &mut config.data_local() {
        Some(dir) => dir.parsed().to_owned(),
        None => current_dir().unwrap_or_else(|_| {
            notification_box(
                "Can't get workdir!",
                "[ CRITICAL FAILURE ]: FAILED TO READ CURRENT WORKING DIRECTORY!",
                no_notifications,
            );
            exit(256);
        }),
    }
}

fn content_files_or_exit(
    config: &openmw_config::OpenMWConfiguration,
    no_notifications: bool,
) -> Vec<String> {
    let content_files = config
        .content_files_iter()
        .map(|plugin| plugin.value_str().to_owned())
        .collect::<Vec<_>>();

    if content_files.is_empty() {
        notification_box(
            "No Plugins!",
            "No plugins were found in openmw.cfg! No lights to fix!",
            no_notifications,
        );
        exit(4);
    }

    content_files
}

fn load_plugins<'a>(
    content_files: &[String],
    light_config: &LightConfig,
    vfs: &'a VFS,
) -> Vec<LoadedPlugin<'a>> {
    content_files
        .par_iter()
        .rev()
        .filter_map(|plugin| {
            let vfs_file = vfs.get_file(plugin.as_str())?;
            let path = vfs_file.path();

            if !is_fixable_plugin(path) || light_config.is_excluded_plugin(path) {
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
        .collect::<Vec<_>>()
}

fn hsv_to_rgb8(hsv: Hsv) -> [u8; 4] {
    let rgb8_color: Srgb<u8> = <Hsv as IntoColor<Srgb>>::into_color(hsv).into_format();
    [rgb8_color.red, rgb8_color.green, rgb8_color.blue, 0]
}

fn apply_cell_ambient_overrides(
    light_config: &LightConfig,
    cell_id: &str,
    atmo: &mut AtmosphereData,
) -> bool {
    let mut replaced = false;

    for (pattern, replacement_data) in &light_config.ambient_regexes {
        if !pattern.is_match(cell_id) {
            continue;
        }

        if let Some(ambient) = &replacement_data.ambient {
            let hsv = Hsv::from_components((
                palette::RgbHue::from_degrees(hue_degrees(ambient.hue)),
                ambient.saturation,
                ambient.value,
            ));

            atmo.ambient_color = hsv_to_rgb8(hsv);
            replaced = true;
        }

        if let Some(fog) = &replacement_data.fog {
            let hsv = Hsv::from_components((
                palette::RgbHue::from_degrees(hue_degrees(fog.hue)),
                fog.saturation,
                fog.value,
            ));

            atmo.fog_color = hsv_to_rgb8(hsv);
            replaced = true;
        }

        if let Some(sunlight) = &replacement_data.sunlight {
            let hsv = Hsv::from_components((
                palette::RgbHue::from_degrees(hue_degrees(sunlight.hue)),
                sunlight.saturation,
                sunlight.value,
            ));

            atmo.sunlight_color = hsv_to_rgb8(hsv);
            replaced = true;
        }

        if let Some(density) = replacement_data.fog_density {
            atmo.fog_density = density;
            replaced = true;
        }
    }

    replaced
}

fn process_cells(
    plugin: &mut Plugin,
    generated_plugin: &mut Plugin,
    light_config: &LightConfig,
    used_ids: &mut HashSet<String>,
) -> u32 {
    let mut used_objects = 0;

    for cell in plugin.objects_of_type_mut::<Cell>().filter(|cell| {
        cell.data.flags.contains(CellFlags::IS_INTERIOR) && cell.atmosphere_data.is_some()
    }) {
        let cell_id = cell.editor_id_ascii_lowercase().into_owned();

        if used_ids.contains(&cell_id) || light_config.is_excluded_id(&cell_id) {
            continue;
        }

        if let Some(ref mut atmo) = cell.atmosphere_data {
            // Need additional handling here for instance replacements!
            // Filter out any instances which are not either in the `deletions` or `replacements` lists.
            cell.references.clear();

            if cell.water_height.is_some() {
                cell.water_height = None;
            }

            let mut replaced = false;

            if light_config.disable_interior_sun {
                atmo.sunlight_color = [0, 0, 0, 0];
                replaced = true;
            }

            replaced |= apply_cell_ambient_overrides(light_config, &cell_id, atmo);

            if replaced {
                generated_plugin.objects.push(take(cell).into());
                used_ids.insert(cell_id);
                used_objects += 1;
            }
        }
    }

    used_objects
}

fn process_lights(
    plugin: Plugin,
    generated_plugin: &mut Plugin,
    light_config: &LightConfig,
    used_ids: &mut HashSet<String>,
) -> u32 {
    let mut used_objects = 0;

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
            process_light(light_config, &mut light);

            generated_plugin.objects.push(light.into());
            used_objects += 1;
        });

    used_objects
}

fn header_for_generated_plugin() -> Header {
    Header {
        version: 1.3,
        author: FixedString("S3".to_string()),
        description: FixedString("Plugin generated by s3-lightfixes".to_string()),
        file_type: FileType::Esp,
        flags: ObjectFlags::default(),
        num_objects: 0,
        masters: Vec::new(),
    }
}

fn plugin_master(plugin_path: &Path, no_notifications: bool) -> io::Result<(String, u64)> {
    let plugin_size = metadata(plugin_path)?.len();
    let Some(name) = plugin_path.file_name() else {
        notification_box(
            "Bad plugin path!",
            "Lightfixes could not resolve the name of one of your plugins! This is UBER Bad and should never happen!",
            no_notifications,
        );
        exit(3);
    };

    Ok((name.to_string_lossy().to_string(), plugin_size))
}

fn generate_plugin(
    plugins: Vec<LoadedPlugin<'_>>,
    light_config: &LightConfig,
) -> io::Result<GenerationResult> {
    let mut plugin = Plugin::new();
    let mut header = header_for_generated_plugin();
    let mut used_ids = HashSet::new();

    for (mut source_plugin, plugin_path) in plugins {
        let used_cell_objects =
            process_cells(&mut source_plugin, &mut plugin, light_config, &mut used_ids);
        let used_light_objects =
            process_lights(source_plugin, &mut plugin, light_config, &mut used_ids);
        let used_objects = used_cell_objects + used_light_objects;

        if used_objects > 0 {
            let (plugin_string, plugin_size) =
                plugin_master(plugin_path, light_config.no_notifications)?;

            header.masters.insert(0, (plugin_string, plugin_size));
            header.num_objects += used_objects;
        }
    }

    Ok(GenerationResult { plugin, header })
}

fn remove_old_plugin_from_data_local(config: &mut openmw_config::OpenMWConfiguration) {
    if let Some(dir) = &mut config.data_local() {
        let old_plug_path = dir.parsed().join(PLUGIN_NAME);
        if old_plug_path.is_file() {
            let _ = remove_file(old_plug_path);
        }
    }
}

fn auto_enable_plugin(config: &mut openmw_config::OpenMWConfiguration, light_config: &LightConfig) {
    if !light_config.auto_enable || config.has_content_file(PLUGIN_NAME) {
        return;
    }

    match config.add_content_file(PLUGIN_NAME) {
        Ok(()) => {
            if let Err(err) = config.save_user() {
                notification_box(
                    "Failed to resave openmw.cfg!",
                    &err.to_string(),
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
            exit(256);
        }
    }
}

fn save_log_if_requested(
    config: &openmw_config::OpenMWConfiguration,
    light_config: &LightConfig,
    generated_plugin: &Plugin,
) -> io::Result<()> {
    if !light_config.save_log {
        return Ok(());
    }

    let path = config.user_config_path().join(LOG_NAME);
    let mut file = File::create(path)?;
    let _ = write!(file, "{generated_plugin:#?}");

    Ok(())
}

/// Runs the command-line application.
///
/// # Errors
///
/// Returns filesystem errors encountered while creating the optional debug log. Configuration,
/// plugin-save, and user-facing validation errors keep the historical notification/exit-code
/// behavior of the binary.
#[allow(clippy::too_many_lines)]
pub fn run() -> io::Result<()> {
    let mut args = LightArgs::parse();

    if args.info {
        println!("S3LightFixes Version: {}", env!("CARGO_PKG_VERSION"));
        exit(0);
    }

    let no_notifications = var("S3L_NO_NOTIFICATIONS").is_ok() || args.no_notifications;
    let mut config = load_openmw_config(&mut args, no_notifications);
    let output_dir = output_dir_from_args_or_config(&args, &mut config, no_notifications);
    let light_config = LightConfig::get(args, &config)?;

    if light_config.debug {
        dbg!(&light_config, &config);
    }

    let content_files = content_files_or_exit(&config, light_config.no_notifications);
    let directories = config
        .data_directories_iter()
        .map(openmw_config::DirectorySetting::parsed)
        .collect::<Vec<_>>();
    let vfs = VFS::from_directories(directories, None);
    let plugins = load_plugins(&content_files, &light_config, &vfs);
    let GenerationResult { mut plugin, header } = generate_plugin(plugins, &light_config)?;

    if light_config.debug {
        dbg!(&header);
    }

    if header.masters.is_empty() {
        notification_box(
            "No masters found!",
            "The generated plugin was not found to have any master files! It's empty! Try running lightfixes again using the S3L_DEBUG environment variable",
            light_config.no_notifications,
        );
        exit(2);
    }

    plugin.objects.push(TES3Object::Header(header));
    plugin.sort_objects();

    // If the old plugin format exists, remove it before serializing the new plugin, as the target
    // dir may still be the old one.
    remove_old_plugin_from_data_local(&mut config);

    if let Err(err) = save_plugin(&output_dir, &mut plugin) {
        notification_box(
            "Failed to save plugin!",
            &err.to_string(),
            light_config.no_notifications,
        );
    }

    auto_enable_plugin(&mut config, &light_config);
    save_log_if_requested(&config, &light_config, &plugin)?;

    let lights_fixed = format!(
        "S3LightFixes.omwaddon generated, enabled, and saved in {}",
        output_dir.display()
    );

    notification_box(
        "Lightfixes successful!",
        &lights_fixed,
        light_config.no_notifications,
    );

    Ok(())
}
