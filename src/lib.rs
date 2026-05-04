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

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        fn new(name: &str) -> Self {
            let (stem, extension) = name
                .rsplit_once('.')
                .map_or((name, ""), |(stem, extension)| (stem, extension));
            let extension = if extension.is_empty() {
                String::new()
            } else {
                format!(".{extension}")
            };
            let path = std::env::temp_dir().join(format!(
                "s3lightfixes-lib-{stem}-{}-{}{extension}",
                std::process::id(),
                NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::write(&path, []).unwrap();

            Self { path }
        }

        fn with_exact_name_in_unique_dir(name: &str) -> Self {
            let directory = std::env::temp_dir().join(format!(
                "s3lightfixes-lib-dir-{}-{}",
                std::process::id(),
                NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&directory).unwrap();
            let path = directory.join(name);
            std::fs::write(&path, []).unwrap();

            Self { path }
        }

        fn as_path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            if let Some(parent) = self.path.parent() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }

    #[test]
    fn is_fixable_plugin_accepts_supported_extensions_case_insensitively() {
        for name in ["mod.esp", "mod.ESM", "mod.OmWaDdOn", "mod.omwgame"] {
            let file = TempFile::new(name);

            assert!(is_fixable_plugin(file.as_path()), "{name}");
        }
    }

    #[test]
    fn is_fixable_plugin_rejects_missing_files_unsupported_extensions_and_generated_plugin() {
        let txt = TempFile::new("mod.txt");
        let generated = TempFile::with_exact_name_in_unique_dir(PLUGIN_NAME);
        let missing = std::env::temp_dir().join(format!(
            "s3lightfixes-lib-missing-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ));

        assert!(!is_fixable_plugin(txt.as_path()));
        assert!(!is_fixable_plugin(generated.as_path()));
        assert!(!is_fixable_plugin(&missing));
    }
}
