use std::{
    env::current_dir,
    fs::{create_dir_all, metadata},
    io,
    path::{Path, PathBuf},
};

pub use openmw_config::OpenMWConfiguration;
pub use tes3::esp::Plugin;

pub mod default;

pub mod light_args;
pub use light_args::LightArgs;

mod light_config;
pub use light_config::LightConfig;

pub const DEFAULT_CONFIG_NAME: &str = "lightconfig.toml";
pub const LOG_NAME: &str = "lightconfig.log";
pub const PLUGIN_NAME: &str = "S3LightFixes.omwaddon";

pub fn get_config_path(args: &mut LightArgs) -> PathBuf {
    if let Some(path) = &args.openmw_cfg {
        if path.is_dir() && path.join("openmw.cfg").is_file() {
            return path.to_owned();
        } else if path.is_file() {
            return path.to_owned();
        }
        panic!("This shit should never ever happen!");
    } else {
        let cwd_cfg = current_dir()
            .expect("Failed to get current directory")
            .join("openmw.cfg");

        if cwd_cfg.is_file() {
            return cwd_cfg;
        }
    }

    openmw_config::default_config_path()
}

pub fn is_excluded_plugin(plugin_path: &Path, light_config: &LightConfig) -> bool {
    let file_name = match plugin_path.file_name() {
        None => return false,
        Some(name) => name.to_string_lossy(),
    };

    for pattern in &light_config.excluded_plugins {
        let reg_from_pattern = regex::Regex::new(&pattern);

        if let Ok(pattern) = reg_from_pattern {
            if pattern.is_match(&file_name) {
                return true;
            }
        }
    }

    false
}

pub fn is_excluded_id(record_id: &str, light_config: &LightConfig) -> bool {
    for pattern in &light_config.excluded_ids {
        let reg_from_pattern = regex::Regex::new(&pattern);

        if let Ok(pattern) = reg_from_pattern {
            if pattern.is_match(record_id) {
                return true;
            }
        }
    }

    false
}

pub fn is_fixable_plugin(plug_path: &Path) -> bool {
    // If path doesn't exist
    if metadata(plug_path).is_err() {
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

/// Displays a notification taking title and message as argument
pub fn notification_box(title: &str, message: &str, no_notifications: bool) {
    #[cfg(target_os = "android")]
    println!("{}", message);

    #[cfg(not(target_os = "android"))]
    if !no_notifications {
        let _ = native_dialog::DialogBuilder::message()
            .set_title(title)
            .set_text(message)
            .alert()
            .show();
    } else {
        println!("{}", message);
    }
}

pub fn save_plugin(output_dir: &PathBuf, generated_plugin: &mut Plugin) -> io::Result<()> {
    let mut plugin_path = output_dir.join(PLUGIN_NAME);

    match metadata(output_dir) {
        Ok(metadata) if !metadata.is_dir() => {
            let cwd =
                current_dir().expect("CRITICAL FAILURE: FAILED TO READ CURRENT WORKING DIRECTORY!");

            eprintln!(
                "WARNING: Couldn't use {} as an output directory, as it isn't a directory. Using the current working directory, {}, instead!",
                output_dir.display(),
                cwd.display()
            );

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

pub fn to_io_error<E: std::fmt::Display>(err: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
}
