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
    CustomCellAmbient, CustomLightData, DEFAULT_CONFIG_NAME, default, notification_box, to_io_error,
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

    #[serde(default = "default::save_log")]
    pub save_log: bool,

    #[serde(default = "default::auto_enable")]
    pub auto_enable: bool,

    #[serde(default)]
    pub no_notifications: bool,

    #[serde(default)]
    pub debug: bool,

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
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.file_name().eq_ignore_ascii_case(DEFAULT_CONFIG_NAME))
            .map(|entry| entry.path())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Light config not found"))
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
    /// use_classic dictates whether or not a fixed radius of 2.0 will be used on orange-y lights
    /// and whether or not to disable interior sunlight
    /// the latter field is not de/serializable and can only be used via the --classic argument
    pub fn get(
        mut light_args: crate::LightArgs,
        openmw_config: &openmw_config::OpenMWConfiguration,
    ) -> Result<LightConfig, io::Error> {
        let mut write_config = false;

        let user_config_path = openmw_config.user_config_path();

        let mut light_config: LightConfig = if let Ok(config_path) = Self::find(&user_config_path) {
            let config_contents = read_to_string(config_path)?;

            match toml::from_str(&config_contents) {
                Ok(config) => config,
                Err(e) => {
                    notification_box(
                        "Failed to read light config!",
                        &format!("Lightconfig.toml couldn't be read: {e}"),
                        light_args.no_notifications,
                    );
                    std::process::exit(256);
                }
            }
        } else {
            write_config = true;
            LightConfig::default()
        };

        // Replace any values provided as CLI args in the config
        // use_classic will always override the standard_radius and disable_interior_sun
        Self::overwrite_if_some([
            (&mut light_config.standard_hue, &mut light_args.standard_hue),
            (
                &mut light_config.standard_saturation,
                &mut light_args.standard_saturation,
            ),
            (
                &mut light_config.standard_value,
                &mut light_args.standard_value,
            ),
            (
                &mut light_config.standard_radius,
                &mut light_args.standard_radius,
            ),
            (&mut light_config.colored_hue, &mut light_args.colored_hue),
            (
                &mut light_config.colored_saturation,
                &mut light_args.colored_saturation,
            ),
            (
                &mut light_config.colored_value,
                &mut light_args.colored_value,
            ),
            (
                &mut light_config.colored_radius,
                &mut light_args.colored_radius,
            ),
            (
                &mut light_config.duration_mult,
                &mut light_args.duration_mult,
            ),
        ]);

        Self::overwrite_if_some([
            (
                &mut light_config.disable_pulse,
                &mut light_args.disable_pulse,
            ),
            (
                &mut light_config.disable_flickering,
                &mut light_args.disable_flickering,
            ),
            (
                &mut light_config.save_log,
                &mut if light_args.write_log {
                    Some(light_args.write_log)
                } else {
                    None
                },
            ),
            (
                &mut light_config.auto_enable,
                &mut if light_args.auto_enable {
                    Some(light_args.auto_enable)
                } else {
                    None
                },
            ),
            (
                &mut light_config.no_notifications,
                &mut if light_args.no_notifications {
                    Some(light_args.no_notifications)
                } else {
                    None
                },
            ),
            (
                &mut light_config.debug,
                &mut if light_args.debug {
                    Some(light_args.debug)
                } else {
                    None
                },
            ),
        ]);

        light_config.no_notifications |= std::env::var("S3L_NO_NOTIFICATIONS").is_ok();
        light_config.debug |= std::env::var("S3L_DEBUG").is_ok();

        // If an output directory was specified via CLI, that should override config options
        // If the provided path is valid
        if let Some(out_dir) = light_args.output {
            if out_dir.is_dir() {
                light_config.output_dir = Some(out_dir);
            } else {
                notification_box(
                    "Can't find output location!",
                    &format!(
                        "WARNING: The requested output path {out_dir:?} does not exist! Terminating."
                    ),
                    light_config.no_notifications,
                );
                std::process::exit(1)
            }
        // Otherwise, if there is neither an output directory specified by the config nor the CLI, use the default location,
        // Being data-local, if defined by the current openmw.cfg, or the current working directory
        } else if let None = light_config.output_dir {
            light_config.output_dir = Some(match openmw_config.data_local() {
                Some(path) => path.parsed().to_owned(),
                None => std::env::current_dir().expect("Failed to get workdir!"),
            });
        };

        light_config
            .excluded_ids
            .extend(std::mem::take(&mut light_args.excluded_ids));

        light_config
            .excluded_plugins
            .extend(std::mem::take(&mut light_args.excluded_plugins));

        light_config
            .light_overrides
            .extend(std::mem::take(&mut light_args.light_overrides));

        light_config
            .ambient_overrides
            .extend(std::mem::take(&mut light_args.ambient_overrides));

        // This parameter indicates whether the user requested
        // To use compatibility mode for vtastek's old 0.47 shaders
        // via startup arguments
        // Drastically increases light radii
        // and disables interior sunlight
        if light_args.use_classic {
            light_config.disable_interior_sun = true;
        }

        // If the configuration file didn't exist when we tried to find it, or the user specified to update
        // serialize it here
        if write_config || light_config.save_config || light_args.update_light_config {
            let config_serialized = toml::to_string_pretty(&light_config).map_err(to_io_error)?;

            let config_path = user_config_path.join(DEFAULT_CONFIG_NAME);
            let mut config_file = File::create(config_path)?;
            write!(config_file, "{}", config_serialized)?;
        }

        // Consume the original values *after* reserializing the config
        std::mem::take(&mut light_config.excluded_ids)
            .into_iter()
            .for_each(|id| {
                if let Ok(pattern) = regex::Regex::new(&id) {
                    light_config.excluded_id_regexes.push(pattern);
                }
            });

        std::mem::take(&mut light_config.excluded_plugins)
            .into_iter()
            .for_each(|id| {
                if let Ok(pattern) = regex::Regex::new(&id.to_ascii_lowercase()) {
                    light_config.excluded_plugin_regexes.push(pattern);
                }
            });

        std::mem::take(&mut light_config.light_overrides)
            .into_iter()
            .for_each(|(id, light_data)| {
                if let Ok(pattern) = regex::Regex::new(&id) {
                    light_config.light_regexes.push((pattern, light_data));
                } // Handle bad patterns and bail here
                // Later
            });

        std::mem::take(&mut light_config.ambient_overrides)
            .into_iter()
            .for_each(|(id, light_data)| {
                if let Ok(pattern) = regex::Regex::new(&id) {
                    light_config.ambient_regexes.push((pattern, light_data));
                } // Handle bad patterns and bail here
                // Later
            });

        Ok(light_config)
    }

    pub fn is_excluded_plugin(&self, plugin_path: &std::path::Path) -> bool {
        let file_name = match plugin_path.file_name() {
            None => return false,
            Some(name) => name.to_ascii_lowercase().into_string().unwrap_or_default(),
        };

        for pattern in &self.excluded_plugin_regexes {
            if pattern.is_match(&file_name) {
                return true;
            }
        }

        false
    }

    pub fn is_excluded_id(&self, record_id: &str) -> bool {
        for pattern in &self.excluded_id_regexes {
            if pattern.is_match(record_id) {
                return true;
            };
        }

        false
    }
}

impl Default for LightConfig {
    fn default() -> LightConfig {
        LightConfig {
            save_config: false,
            debug: false,
            no_notifications: false,
            output_dir: None,
            disable_interior_sun: false,
            disable_flickering: default::disable_flicker(),
            disable_pulse: default::disable_pulse(),
            save_log: default::save_log(),
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
