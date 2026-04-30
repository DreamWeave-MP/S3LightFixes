use std::{
    env::current_dir,
    fs::{create_dir_all, metadata},
    io,
    path::{Path, PathBuf},
};

pub use openmw_config::OpenMWConfiguration;
pub use tes3::esp::Plugin;

mod app;
pub use app::run;

pub mod default;

pub mod light_args;
pub use light_args::LightArgs;

mod light_config;
pub use light_config::LightConfig;

mod light_override;
pub use light_override::{CustomCellAmbient, CustomLightData};

mod light_processing;
pub use light_processing::{light_to_hsv, process_light};

pub const DEFAULT_CONFIG_NAME: &str = "lightconfig.toml";
pub const LOG_NAME: &str = "lightconfig.log";
pub const PLUGIN_NAME: &str = "S3LightFixes.omwaddon";

/// Returns the `openmw.cfg` path selected by CLI arguments, the current directory, or platform defaults.
///
/// # Panics
///
/// Panics if the user explicitly supplied an `openmw.cfg` path that resolves to neither a file nor
/// a directory containing `openmw.cfg`. That is a caller contract violation; continuing would make
/// the rest of the run operate on a different config than requested.
pub fn get_config_path(args: &mut LightArgs) -> PathBuf {
    if let Some(path) = &args.openmw_cfg {
        let absolute_path = if path.is_relative() {
            path.canonicalize().unwrap_or_else(|_| path.to_owned())
        } else {
            path.to_owned()
        };

        if absolute_path.is_file()
            || (absolute_path.is_dir() && absolute_path.join("openmw.cfg").is_file())
        {
            return absolute_path;
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

#[must_use]
pub fn is_fixable_plugin(plug_path: &Path) -> bool {
    metadata(plug_path).is_ok()
        && !plug_path.to_string_lossy().contains(PLUGIN_NAME)
        && plug_path.extension().is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().to_str().unwrap_or_default(),
                "esp" | "esm" | "omwaddon" | "omwgame"
            )
        })
}

/// Displays a notification taking title and message as argument
pub fn notification_box(title: &str, message: &str, no_notifications: bool) {
    #[cfg(target_os = "android")]
    println!("{message}");

    #[cfg(not(target_os = "android"))]
    if no_notifications {
        println!("{message}");
    } else {
        let _ = native_dialog::DialogBuilder::message()
            .set_title(title)
            .set_text(message)
            .alert()
            .show();
    }
}

/// Saves the generated plugin to the requested output directory.
///
/// # Errors
///
/// Returns any filesystem error encountered while creating the output directory, resolving the
/// fallback current directory, or writing the plugin file.
pub fn save_plugin(output_dir: &PathBuf, generated_plugin: &mut Plugin) -> io::Result<()> {
    let mut plugin_path = output_dir.join(PLUGIN_NAME);

    match metadata(output_dir) {
        Ok(metadata) if !metadata.is_dir() => {
            let cwd = current_dir()?;

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
