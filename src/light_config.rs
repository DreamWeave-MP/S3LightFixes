use std::{
    fmt,
    fs::{File, read_dir, read_to_string},
    io::{self, Write},
    marker::PhantomData,
    path::PathBuf,
};

use ordered_hash_map::OrderedHashMap;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
};

use crate::{
    CustomCellAmbient, CustomLightData, DEFAULT_CONFIG_NAME, LightArgs, default, notification_box,
    to_io_error,
};

pub fn deserialize_ordered_hash_map<'de, D, K, V>(
    deserializer: D,
) -> Result<OrderedHashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + std::hash::Hash,
    V: Deserialize<'de>,
{
    struct OrderedHashMapVisitor<K, V> {
        marker: PhantomData<fn() -> OrderedHashMap<K, V>>,
    }

    impl<K, V> OrderedHashMapVisitor<K, V> {
        fn new() -> Self {
            OrderedHashMapVisitor {
                marker: PhantomData,
            }
        }
    }

    impl<'de, K, V> Visitor<'de> for OrderedHashMapVisitor<K, V>
    where
        K: Deserialize<'de> + Eq + std::hash::Hash,
        V: Deserialize<'de>,
    {
        type Value = OrderedHashMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = OrderedHashMap::with_capacity(access.size_hint().unwrap_or(0));

            while let Some((key, value)) = access.next_entry()? {
                map.insert(key, value);
            }

            Ok(map)
        }
    }

    deserializer.deserialize_map(OrderedHashMapVisitor::new())
}

pub fn serialize_ordered_hash_map<S, K, V>(
    map: &OrderedHashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Serialize,
    V: Serialize,
{
    use serde::ser::SerializeMap;

    let mut ser_map = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        ser_map.serialize_entry(k, v)?;
    }
    ser_map.end()
}

#[derive(Debug, Deserialize, Serialize)]
// This struct is the public TOML schema. Grouping the booleans into enum wrappers would either
// change the config format or add serde indirection that exists only to satisfy clippy.
#[allow(clippy::struct_excessive_bools)]
pub struct LightConfig {
    /// This parameter is DANGEROUS
    /// It's only meant to be used with vtastek's experimental shaders for openmw 0.47
    /// <https://discord.com/channels/260439894298460160/718892786157617163/966468825321177148>
    #[serde(default)]
    pub disable_interior_sun: bool,

    #[serde(default = "default::disable_flicker")]
    pub disable_flickering: bool,

    #[serde(default = "default::disable_pulse")]
    pub disable_pulse: bool,

    #[serde(default = "default::disable_negative_lights")]
    pub disable_negative_lights: bool,

    #[serde(default = "default::auto_enable")]
    pub auto_enable: bool,

    #[serde(default)]
    pub no_notifications: bool,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub dry_run: bool,

    #[serde(default)]
    pub validate_config: bool,

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

    #[serde(default = "default::excluded_plugins")]
    pub excluded_plugins: Vec<String>,

    #[serde(default)]
    pub excluded_ids: Vec<String>,

    #[serde(
        default,
        serialize_with = "serialize_ordered_hash_map",
        deserialize_with = "deserialize_ordered_hash_map"
    )]
    pub light_overrides: OrderedHashMap<String, CustomLightData>,

    #[serde(
        default,
        serialize_with = "serialize_ordered_hash_map",
        deserialize_with = "deserialize_ordered_hash_map"
    )]
    pub ambient_overrides: OrderedHashMap<String, CustomCellAmbient>,

    pub output_dir: Option<PathBuf>,

    #[serde(default)]
    pub save_config: bool,

    #[serde(skip)]
    pub excluded_id_regexes: Vec<regex::Regex>,
    #[serde(skip)]
    pub excluded_plugin_regexes: Vec<regex::Regex>,
    #[serde(skip)]
    pub light_regexes: Vec<(regex::Regex, CustomLightData)>,
    #[serde(skip)]
    pub ambient_regexes: Vec<(regex::Regex, CustomCellAmbient)>,
}

/// Primarily exists to provide default implementations
/// for field values
impl LightConfig {
    fn find(root_path: &PathBuf) -> Result<PathBuf, io::Error> {
        read_dir(root_path)?
            .filter_map(std::result::Result::ok)
            .find(|entry| entry.file_name().eq_ignore_ascii_case(DEFAULT_CONFIG_NAME))
            .map(|entry| entry.path())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Light config not found"))
    }

    fn load(
        user_config_path: &std::path::Path,
        early_no_notifications: bool,
    ) -> io::Result<(Self, bool)> {
        let Ok(config_path) = Self::find(&user_config_path.to_path_buf()) else {
            return Ok((LightConfig::default(), true));
        };

        let config_contents = read_to_string(config_path)?;
        let config = toml::from_str(&config_contents).unwrap_or_else(|error| {
            notification_box(
                "Failed to read light config!",
                &format!("Lightconfig.toml couldn't be read: {error}"),
                early_no_notifications,
            );
            std::process::exit(256);
        });

        Ok((config, false))
    }

    fn apply_scalar_args(&mut self, light_args: &mut LightArgs) {
        Self::overwrite_if_some([
            (&mut self.standard_hue, &mut light_args.standard_hue),
            (
                &mut self.standard_saturation,
                &mut light_args.standard_saturation,
            ),
            (&mut self.standard_value, &mut light_args.standard_value),
            (&mut self.standard_radius, &mut light_args.standard_radius),
            (&mut self.colored_hue, &mut light_args.colored_hue),
            (
                &mut self.colored_saturation,
                &mut light_args.colored_saturation,
            ),
            (&mut self.colored_value, &mut light_args.colored_value),
            (&mut self.colored_radius, &mut light_args.colored_radius),
            (&mut self.duration_mult, &mut light_args.duration_mult),
        ]);
    }

    fn apply_bool_args(&mut self, light_args: &LightArgs) {
        Self::overwrite_if_some([
            (
                &mut self.disable_pulse,
                &mut light_args.disable_pulse.clone(),
            ),
            (
                &mut self.disable_flickering,
                &mut light_args.disable_flickering.clone(),
            ),
            (
                &mut self.disable_negative_lights,
                &mut light_args.disable_negative_lights.clone(),
            ),
            (
                &mut self.auto_enable,
                &mut light_args.auto_enable.then_some(true),
            ),
            (
                &mut self.no_notifications,
                &mut light_args.no_notifications.then_some(true),
            ),
            (&mut self.debug, &mut light_args.debug.then_some(true)),
        ]);

        if let Some(dry_run) = light_args.dry_run {
            self.dry_run = dry_run;
            if dry_run {
                self.validate_config = false;
            }
        }

        if let Some(validate_config) = light_args.validate_config {
            self.validate_config = validate_config;
            if validate_config {
                self.dry_run = false;
            }
        }
    }

    fn effective_non_writing_modes(&self, light_args: &LightArgs) -> io::Result<(bool, bool)> {
        let mut dry_run = self.dry_run;
        let mut validate_config = self.validate_config;

        if let Some(cli_dry_run) = light_args.dry_run {
            dry_run = cli_dry_run;
            if cli_dry_run {
                validate_config = false;
            }
        }

        if let Some(cli_validate_config) = light_args.validate_config {
            validate_config = cli_validate_config;
            if cli_validate_config {
                dry_run = false;
            }
        }

        if dry_run && validate_config {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "dry_run and validate_config cannot both be true",
            ));
        }

        Ok((dry_run, validate_config))
    }

    fn apply_collection_args(&mut self, light_args: &mut LightArgs) {
        self.excluded_ids
            .extend(std::mem::take(&mut light_args.excluded_ids));
        self.excluded_plugins
            .extend(std::mem::take(&mut light_args.excluded_plugins));
        self.light_overrides
            .extend(std::mem::take(&mut light_args.light_overrides));
        self.ambient_overrides
            .extend(std::mem::take(&mut light_args.ambient_overrides));
    }

    fn configure_output_dir(
        &mut self,
        output_dir: Option<PathBuf>,
        openmw_config: &openmw_config::OpenMWConfiguration,
    ) -> io::Result<()> {
        if let Some(out_dir) = output_dir {
            if out_dir.is_dir() {
                self.output_dir = Some(out_dir);
                return Ok(());
            }

            notification_box(
                "Can't find output location!",
                &format!(
                    "WARNING: The requested output path {} does not exist! Terminating.",
                    out_dir.display()
                ),
                self.no_notifications,
            );
            std::process::exit(1);
        }

        if self.output_dir.is_none() {
            self.output_dir = Some(match openmw_config.data_local() {
                Some(path) => path.parsed().to_owned(),
                None => std::env::current_dir()?,
            });
        }

        Ok(())
    }

    fn save_to_user_config(&self, user_config_path: &std::path::Path) -> io::Result<()> {
        let config_serialized = toml::to_string_pretty(self).map_err(to_io_error)?;
        let config_path = user_config_path.join(DEFAULT_CONFIG_NAME);
        let mut config_file = File::create(config_path)?;
        write!(config_file, "{config_serialized}")
    }

    fn save_to_user_config_without_runtime_flags(
        &mut self,
        user_config_path: &std::path::Path,
        persisted_no_notifications: bool,
        persisted_debug: bool,
    ) -> io::Result<()> {
        let runtime_no_notifications =
            std::mem::replace(&mut self.no_notifications, persisted_no_notifications);
        let runtime_debug = std::mem::replace(&mut self.debug, persisted_debug);
        let result = self.save_to_user_config(user_config_path);
        self.no_notifications = runtime_no_notifications;
        self.debug = runtime_debug;

        result
    }

    fn compile_regexes(&mut self) -> io::Result<()> {
        let mut errors = Vec::new();

        for id in std::mem::take(&mut self.excluded_ids) {
            match regex::RegexBuilder::new(&id).case_insensitive(true).build() {
                Ok(pattern) => self.excluded_id_regexes.push(pattern),
                Err(error) => {
                    let message = format!("Couldn't compile excluded id regex: {id}: {error}");
                    notification_box(
                        "Invalid excluded id regex!",
                        &message,
                        self.no_notifications,
                    );
                    errors.push(message);
                }
            }
        }

        for id in std::mem::take(&mut self.excluded_plugins) {
            match regex::RegexBuilder::new(&id).case_insensitive(true).build() {
                Ok(pattern) => self.excluded_plugin_regexes.push(pattern),
                Err(error) => {
                    let message = format!("Couldn't compile excluded plugin regex: {id}: {error}");
                    notification_box(
                        "Invalid excluded plugin regex!",
                        &message,
                        self.no_notifications,
                    );
                    errors.push(message);
                }
            }
        }

        for (id, light_data) in std::mem::take(&mut self.light_overrides) {
            match regex::Regex::new(&id) {
                Ok(pattern) => self.light_regexes.push((pattern, light_data)),
                Err(error) => {
                    let message = format!("Couldn't compile light override regex: {id}: {error}");
                    notification_box("Invalid light override!", &message, self.no_notifications);
                    errors.push(message);
                }
            }
        }

        for (id, light_data) in std::mem::take(&mut self.ambient_overrides) {
            match regex::Regex::new(&id) {
                Ok(pattern) => self.ambient_regexes.push((pattern, light_data)),
                Err(error) => {
                    let message = format!("Couldn't compile ambient override regex: {id}: {error}");
                    notification_box("Invalid ambient override!", &message, self.no_notifications);
                    errors.push(message);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                errors.join("\n"),
            ))
        }
    }

    fn overwrite_if_some<'a, I, T>(pairs: I)
    where
        // (&mut T, &mut Option<T>) for every element
        I: IntoIterator<Item = (&'a mut T, &'a mut Option<T>)>,
        // Restrict to primitive / scalar types
        T: Copy + Default + 'a,
    {
        for (field, maybe_val) in pairs {
            if let Some(v) = maybe_val {
                *field = std::mem::take(v); // move value across, leave default behind
            }
        }
    }

    /// Gives back the lightconfig adjacent to openmw.cfg when called
    /// `use_classic` dictates whether or not a fixed radius of 2.0 will be used on orange-y lights
    /// and whether or not to disable interior sunlight
    /// the latter field is not de/serializable and can only be used via the --classic argument
    ///
    /// # Errors
    ///
    /// Returns filesystem errors encountered while reading or writing `lightconfig.toml`, or while
    /// resolving the fallback output directory.
    pub fn get(
        mut light_args: LightArgs,
        openmw_config: &openmw_config::OpenMWConfiguration,
    ) -> Result<LightConfig, io::Error> {
        let user_config_path = openmw_config.user_config_path();

        let early_no_notifications =
            std::env::var("S3L_NO_NOTIFICATIONS").is_ok() || light_args.no_notifications;
        let (mut light_config, write_config) =
            Self::load(&user_config_path, early_no_notifications)?;

        let debug_from_env = std::env::var("S3L_DEBUG").is_ok();

        let (effective_dry_run, effective_validate_config) =
            light_config.effective_non_writing_modes(&light_args)?;
        let allow_config_writes = !effective_dry_run && !effective_validate_config;

        light_config.apply_scalar_args(&mut light_args);
        light_config.apply_bool_args(&light_args);
        let persisted_no_notifications = light_config.no_notifications;
        let persisted_debug = light_config.debug;
        light_config.no_notifications |= std::env::var("S3L_NO_NOTIFICATIONS").is_ok();
        light_config.debug |= debug_from_env;

        if !effective_validate_config {
            light_config.configure_output_dir(light_args.output.take(), openmw_config)?;
        }
        light_config.apply_collection_args(&mut light_args);

        // This parameter indicates whether the user requested
        // To use compatibility mode for vtastek's old 0.47 shaders
        // via startup arguments
        // Drastically increases light radii
        // and disables interior sunlight
        if light_args.use_classic {
            light_config.disable_interior_sun = true;
        }

        if allow_config_writes
            && (write_config || light_config.save_config || light_args.update_light_config)
        {
            light_config.save_to_user_config_without_runtime_flags(
                &user_config_path,
                persisted_no_notifications,
                persisted_debug,
            )?;
        }

        // Consume the original values *after* reserializing the config
        light_config.compile_regexes()?;

        Ok(light_config)
    }

    #[must_use]
    pub fn is_excluded_plugin(&self, plugin_path: &std::path::Path) -> bool {
        let file_name = match plugin_path.file_name() {
            None => return false,
            Some(name) => name.to_string_lossy(),
        };

        for pattern in &self.excluded_plugin_regexes {
            if pattern.is_match(&file_name) {
                return true;
            }
        }

        false
    }

    #[must_use]
    pub fn is_excluded_id(&self, record_id: &str) -> bool {
        for pattern in &self.excluded_id_regexes {
            if pattern.is_match(record_id) {
                return true;
            }
        }

        false
    }
}

impl Default for LightConfig {
    fn default() -> LightConfig {
        LightConfig {
            save_config: false,
            debug: false,
            dry_run: false,
            validate_config: false,
            no_notifications: false,
            output_dir: None,
            disable_interior_sun: false,
            disable_flickering: default::disable_flicker(),
            disable_pulse: default::disable_pulse(),
            disable_negative_lights: default::disable_negative_lights(),
            auto_enable: default::auto_enable(),
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
            excluded_id_regexes: Vec::new(),
            excluded_plugin_regexes: Vec::new(),
            light_regexes: Vec::new(),
            light_overrides: OrderedHashMap::new(),
            ambient_overrides: OrderedHashMap::new(),
            ambient_regexes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use clap::Parser;

    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("s3lightfixes-config-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir(&path).unwrap();

            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn toml_light_overrides_preserve_declaration_order() {
        let config = toml::from_str::<LightConfig>(
            r"
            [light_overrides.first]
            radius = 1

            [light_overrides.second]
            radius = 2

            [light_overrides.third]
            radius = 3
            ",
        )
        .unwrap();

        assert_eq!(
            config.light_overrides.keys().collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
    }

    #[test]
    fn disable_negative_lights_defaults_true_and_can_be_loaded_false() {
        let defaulted = toml::from_str::<LightConfig>("").unwrap();
        assert!(defaulted.disable_negative_lights);

        let configured = toml::from_str::<LightConfig>("disable_negative_lights = false").unwrap();
        assert!(!configured.disable_negative_lights);
    }

    #[test]
    fn cli_disable_negative_lights_overrides_config_without_short_form() {
        let mut config = LightConfig {
            disable_negative_lights: true,
            ..LightConfig::default()
        };
        let args = LightArgs::parse_from([
            "s3lightfixes",
            "--disable-negative-lights",
            "false",
            "--no-flicker",
            "false",
        ]);

        config.apply_bool_args(&args);

        assert!(!config.disable_negative_lights);
        assert!(!config.disable_flickering);
    }

    #[test]
    fn non_writing_modes_are_toml_configurable_and_cli_overridable() {
        let mut config = toml::from_str::<LightConfig>(
            r"
            dry_run = true
            validate_config = false
            ",
        )
        .unwrap();
        let args = LightArgs::parse_from(["s3lightfixes", "--validate-config"]);

        config.apply_bool_args(&args);

        assert!(!config.dry_run);
        assert!(config.validate_config);
    }

    #[test]
    fn cli_false_can_disable_configured_non_writing_mode() {
        let mut config = LightConfig {
            dry_run: true,
            ..LightConfig::default()
        };
        let args = LightArgs::parse_from(["s3lightfixes", "--dry-run", "false"]);

        config.apply_bool_args(&args);

        assert!(!config.dry_run);
    }

    #[test]
    fn configured_dry_run_and_validate_config_conflict_without_cli_override() {
        let config = LightConfig {
            dry_run: true,
            validate_config: true,
            ..LightConfig::default()
        };
        let args = LightArgs::parse_from(["s3lightfixes"]);

        let err = config.effective_non_writing_modes(&args).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn invalid_regexes_are_validation_errors_instead_of_silent_drops() {
        let mut config = LightConfig {
            excluded_ids: vec!["[".to_owned()],
            no_notifications: true,
            ..LightConfig::default()
        };

        let err = config.compile_regexes().unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(
            err.to_string()
                .contains("Couldn't compile excluded id regex")
        );
    }

    #[test]
    fn legacy_hsv_light_colors_are_preserved() {
        let config = toml::from_str::<LightConfig>(
            r"
            [light_overrides.torch]
            hue = 180
            saturation = 1.0
            value = 1.0
            ",
        )
        .unwrap();

        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("hue = 180"));
        assert!(serialized.contains("saturation = 1.0"));
        assert!(serialized.contains("value = 1.0"));
        assert!(!serialized.contains("red ="));
    }

    #[test]
    fn final_save_strips_env_only_notification_and_debug_state() {
        let temp_dir = TempDir::new("final-save-strips-runtime-state");
        let mut config = LightConfig {
            no_notifications: true,
            debug: true,
            ..LightConfig::default()
        };

        config
            .save_to_user_config_without_runtime_flags(temp_dir.path(), false, false)
            .unwrap();

        assert!(config.no_notifications);
        assert!(config.debug);

        let saved = std::fs::read_to_string(temp_dir.path().join(DEFAULT_CONFIG_NAME)).unwrap();
        assert!(!saved.contains("no_notifications = true"));
        assert!(!saved.contains("debug = true"));
    }
}
