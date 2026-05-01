use std::{
    collections::HashSet,
    env::var,
    fs::{File, copy, metadata, remove_file},
    io::{self, Write},
    mem::take,
    path::{Path, PathBuf},
    process::exit,
};

use clap::{CommandFactory, Parser};
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

use crate::light_processing::process_light;

type LoadedPlugin<'a> = (Plugin, &'a Path);

struct GenerationResult {
    plugin: Plugin,
    header: Header,
    logs: Vec<RecordLog>,
}

#[derive(Debug, PartialEq, Eq)]
struct RecordLog {
    kind: &'static str,
    plugin: String,
    id: String,
    changes: Vec<String>,
}

struct RunMetadata {
    version: &'static str,
    config_path: PathBuf,
    output_path: PathBuf,
    content_files: usize,
    loaded_plugins: usize,
    masters: usize,
    changed_cells: usize,
    changed_lights: usize,
}

impl RunMetadata {
    fn new(
        selected_config_file: &Path,
        output_dir: &Path,
        content_files: usize,
        loaded_plugins: usize,
        header: &Header,
        logs: &[RecordLog],
    ) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            config_path: selected_config_file.to_owned(),
            output_path: output_dir.join(PLUGIN_NAME),
            content_files,
            loaded_plugins,
            masters: header.masters.len(),
            changed_cells: logs.iter().filter(|log| log.kind == "CELL").count(),
            changed_lights: logs.iter().filter(|log| log.kind == "LIGH").count(),
        }
    }
}

fn selected_config_file_path(args: &mut LightArgs) -> PathBuf {
    let config_path = get_config_path(args);

    if config_path.is_dir() {
        config_path.join("openmw.cfg")
    } else {
        config_path
    }
}

fn plugin_log_name(plugin_path: &Path) -> String {
    plugin_path.file_name().map_or_else(
        || plugin_path.display().to_string(),
        |name| name.to_string_lossy().to_string(),
    )
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
            atmo.ambient_color = ambient.to_esp_color();
            replaced = true;
        }

        if let Some(fog) = &replacement_data.fog {
            atmo.fog_color = fog.to_esp_color();
            replaced = true;
        }

        if let Some(sunlight) = &replacement_data.sunlight {
            atmo.sunlight_color = sunlight.to_esp_color();
            replaced = true;
        }

        if let Some(density) = replacement_data.fog_density {
            atmo.fog_density = density;
            replaced = true;
        }
    }

    replaced
}

fn cell_changes(original: &AtmosphereData, modified: &AtmosphereData) -> Vec<String> {
    let mut changes = Vec::new();

    if original.ambient_color != modified.ambient_color {
        changes.push(format!(
            "ambient {:?} -> {:?}",
            original.ambient_color, modified.ambient_color
        ));
    }

    if original.sunlight_color != modified.sunlight_color {
        changes.push(format!(
            "sunlight {:?} -> {:?}",
            original.sunlight_color, modified.sunlight_color
        ));
    }

    if original.fog_color != modified.fog_color {
        changes.push(format!(
            "fog {:?} -> {:?}",
            original.fog_color, modified.fog_color
        ));
    }

    if (original.fog_density - modified.fog_density).abs() > f32::EPSILON {
        changes.push(format!(
            "fog_density {} -> {}",
            original.fog_density, modified.fog_density
        ));
    }

    changes
}

fn process_cells(
    plugin: &mut Plugin,
    plugin_name: &str,
    generated_plugin: &mut Plugin,
    light_config: &LightConfig,
    used_ids: &mut HashSet<String>,
    logs: &mut Vec<RecordLog>,
) -> u32 {
    let mut used_objects = 0;

    for cell in plugin.objects_of_type_mut::<Cell>().filter(|cell| {
        cell.data.flags.contains(CellFlags::IS_INTERIOR) && cell.atmosphere_data.is_some()
    }) {
        let cell_id = cell.editor_id_ascii_lowercase().into_owned();

        if used_ids.contains(&cell_id) || light_config.is_excluded_id(&cell_id) {
            continue;
        }

        let original_atmo = cell.atmosphere_data.clone();
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
                if let Some(original_atmo) = &original_atmo {
                    let changes = cell_changes(original_atmo, atmo);

                    if !changes.is_empty() {
                        logs.push(RecordLog {
                            kind: "CELL",
                            plugin: plugin_name.to_owned(),
                            id: cell_id.clone(),
                            changes,
                        });
                    }
                }

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
    plugin_name: &str,
    generated_plugin: &mut Plugin,
    light_config: &LightConfig,
    used_ids: &mut HashSet<String>,
    logs: &mut Vec<RecordLog>,
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
            let changes = process_light(light_config, &mut light);

            if !changes.is_empty() {
                logs.push(RecordLog {
                    kind: "LIGH",
                    plugin: plugin_name.to_owned(),
                    id: light.id.clone(),
                    changes,
                });
            }

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
    let mut logs = Vec::new();

    for (mut source_plugin, plugin_path) in plugins {
        let plugin_name = plugin_log_name(plugin_path);
        let used_cell_objects = process_cells(
            &mut source_plugin,
            &plugin_name,
            &mut plugin,
            light_config,
            &mut used_ids,
            &mut logs,
        );
        let used_light_objects = process_lights(
            source_plugin,
            &plugin_name,
            &mut plugin,
            light_config,
            &mut used_ids,
            &mut logs,
        );
        let used_objects = used_cell_objects + used_light_objects;

        if used_objects > 0 {
            let (plugin_string, plugin_size) =
                plugin_master(plugin_path, light_config.no_notifications)?;

            header.masters.insert(0, (plugin_string, plugin_size));
            header.num_objects += used_objects;
        }
    }

    Ok(GenerationResult {
        plugin,
        header,
        logs,
    })
}

fn remove_old_plugin_from_data_local(config: &mut openmw_config::OpenMWConfiguration) {
    if let Some(dir) = &mut config.data_local() {
        let old_plug_path = dir.parsed().join(PLUGIN_NAME);
        if old_plug_path.is_file() {
            let _ = remove_file(old_plug_path);
        }
    }
}

fn backup_openmw_cfg(selected_config_file: &Path) -> io::Result<PathBuf> {
    let file_name = selected_config_file.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected OpenMW config path has no file name",
        )
    })?;
    let backup_name = format!("{}.s3lightfixes.bak", file_name.to_string_lossy());
    let backup_path = selected_config_file.with_file_name(backup_name);

    copy(selected_config_file, &backup_path)?;

    Ok(backup_path)
}

fn auto_enable_plugin(
    config: &mut openmw_config::OpenMWConfiguration,
    light_config: &LightConfig,
    selected_config_file: &Path,
) -> bool {
    if !light_config.auto_enable {
        return false;
    }

    if config.has_content_file(PLUGIN_NAME) {
        return true;
    }

    let backup_path = match backup_openmw_cfg(selected_config_file) {
        Ok(path) => path,
        Err(err) => {
            notification_box(
                "Failed to back up openmw.cfg!",
                &format!(
                    "Refusing to auto-enable {PLUGIN_NAME} because openmw.cfg could not be backed up: {err}"
                ),
                light_config.no_notifications,
            );
            return false;
        }
    };

    match config.add_content_file(PLUGIN_NAME) {
        Ok(()) => {
            if let Err(err) = config.save_user() {
                notification_box(
                    "Failed to resave openmw.cfg!",
                    &err.to_string(),
                    light_config.no_notifications,
                );
                false
            } else {
                let lightfix_enabled_msg = format!(
                    "Wrote selected OpenMW config at {} successfully! Backup saved at {}.",
                    selected_config_file.display(),
                    backup_path.display()
                );
                notification_box(
                    "Lightfixes enabled!",
                    &lightfix_enabled_msg,
                    light_config.no_notifications,
                );
                true
            }
        }
        Err(err) => {
            eprintln!("{err}");
            exit(256);
        }
    }
}

fn write_log_outputs(
    config: &openmw_config::OpenMWConfiguration,
    metadata: &RunMetadata,
    logs: &[RecordLog],
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    match write_log_to(&mut stdout, metadata, logs) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => {}
        Err(err) => return Err(err),
    }

    let path = config.user_config_path().join(LOG_NAME);
    let mut file = File::create(path)?;
    write_log_to(&mut file, metadata, logs)
}

fn write_dry_run_outputs(metadata: &RunMetadata, logs: &[RecordLog]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_dry_run_to(&mut stdout, metadata, logs)
}

fn write_dry_run_to(
    mut writer: impl Write,
    metadata: &RunMetadata,
    logs: &[RecordLog],
) -> io::Result<()> {
    writeln!(writer, "Dry run: no files written")?;
    write_log_to(&mut writer, metadata, logs)
}

fn write_log_to(
    mut writer: impl Write,
    metadata: &RunMetadata,
    logs: &[RecordLog],
) -> io::Result<()> {
    writeln!(writer, "# S3LightFixes {}", metadata.version)?;
    writeln!(writer, "# config: {}", metadata.config_path.display())?;
    writeln!(writer, "# output: {}", metadata.output_path.display())?;
    writeln!(writer, "# content files: {}", metadata.content_files)?;
    writeln!(writer, "# loaded plugins: {}", metadata.loaded_plugins)?;
    writeln!(writer, "# masters: {}", metadata.masters)?;
    writeln!(writer, "# changed cells: {}", metadata.changed_cells)?;
    writeln!(writer, "# changed lights: {}", metadata.changed_lights)?;

    for log in logs {
        writeln!(
            writer,
            "{} {:?} from {:?}: {}",
            log.kind,
            log.id,
            log.plugin,
            log.changes.join(", ")
        )?;
    }

    Ok(())
}

fn handle_generated_output(args: &LightArgs, stdout: &mut dyn Write) -> io::Result<bool> {
    if let Some(shell) = args.generate_completion {
        let mut command = LightArgs::command();
        clap_complete::generate(shell, &mut command, "s3lightfixes", stdout);
        return Ok(true);
    }

    if args.generate_manpage {
        clap_mangen::Man::new(LightArgs::command()).render(stdout)?;
        return Ok(true);
    }

    Ok(false)
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

    if handle_generated_output(&args, &mut io::stdout())? {
        return Ok(());
    }

    let no_notifications = var("S3L_NO_NOTIFICATIONS").is_ok() || args.no_notifications;
    let selected_config_file = selected_config_file_path(&mut args);
    let mut config = load_openmw_config(&mut args, no_notifications);
    let light_config = LightConfig::get(args, &config)?;

    if light_config.validate_config {
        println!(
            "Validated {} successfully",
            config
                .user_config_path()
                .join(crate::DEFAULT_CONFIG_NAME)
                .display()
        );
        return Ok(());
    }

    let output_dir = light_config.output_dir.clone().unwrap_or_else(|| {
        notification_box(
            "Can't get output directory!",
            "[ CRITICAL FAILURE ]: FAILED TO RESOLVE OUTPUT DIRECTORY!",
            light_config.no_notifications,
        );
        exit(256);
    });

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
    let loaded_plugins = plugins.len();
    let GenerationResult {
        mut plugin,
        header,
        logs,
    } = generate_plugin(plugins, &light_config)?;

    if light_config.debug {
        dbg!(&header);
    }

    let metadata = RunMetadata::new(
        &selected_config_file,
        &output_dir,
        content_files.len(),
        loaded_plugins,
        &header,
        &logs,
    );

    if header.masters.is_empty() {
        if light_config.dry_run {
            write_dry_run_outputs(&metadata, &logs)?;
            return Ok(());
        }

        notification_box(
            "No masters found!",
            "The generated plugin was not found to have any master files! It's empty! Try running lightfixes again using the S3L_DEBUG environment variable",
            light_config.no_notifications,
        );
        exit(2);
    }

    if light_config.dry_run {
        write_dry_run_outputs(&metadata, &logs)?;
        return Ok(());
    }

    plugin.objects.push(TES3Object::Header(header));
    plugin.sort_objects();

    // If the old plugin format exists, remove it before serializing the new plugin, as the target
    // dir may still be the old one.
    remove_old_plugin_from_data_local(&mut config);

    save_plugin(&output_dir, &mut plugin).inspect_err(|err| {
        notification_box(
            "Failed to save plugin!",
            &err.to_string(),
            light_config.no_notifications,
        );
    })?;

    let enabled = auto_enable_plugin(&mut config, &light_config, &selected_config_file);
    write_log_outputs(&config, &metadata, &logs)?;

    let lights_fixed = if enabled {
        format!(
            "S3LightFixes.omwaddon generated, enabled, and saved in {}",
            output_dir.display()
        )
    } else {
        format!(
            "S3LightFixes.omwaddon generated and saved in {}",
            output_dir.display()
        )
    };

    notification_box(
        "Lightfixes successful!",
        &lights_fixed,
        light_config.no_notifications,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use regex::Regex;
    use tes3::esp::{AtmosphereData, CellData, LightData, LightFlags, Reference, TES3Object};

    use super::*;
    use crate::{CustomCellAmbient, light_override::TypedLightColor};

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempPluginFile {
        path: PathBuf,
    }

    impl TempPluginFile {
        fn as_path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempPluginFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn temp_plugin_file(name: &str, size: usize) -> TempPluginFile {
        let path = std::env::temp_dir().join(format!(
            "s3lightfixes-{name}-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, vec![0; size]).unwrap();

        TempPluginFile { path }
    }

    fn light(id: &str, radius: u32) -> Light {
        Light {
            id: id.to_owned(),
            data: LightData {
                radius,
                time: 10,
                color: [255, 128, 0, 0],
                flags: LightFlags::default(),
                ..LightData::default()
            },
            ..Light::default()
        }
    }

    fn plugin_with_lights(lights: impl IntoIterator<Item = Light>) -> Plugin {
        Plugin {
            objects: lights.into_iter().map(TES3Object::from).collect(),
        }
    }

    fn generated_lights(plugin: &Plugin) -> Vec<&Light> {
        plugin.objects_of_type::<Light>().collect()
    }

    fn generated_cells(plugin: &Plugin) -> Vec<&Cell> {
        plugin.objects_of_type::<Cell>().collect()
    }

    fn config() -> LightConfig {
        LightConfig {
            standard_hue: 1.0,
            standard_saturation: 1.0,
            standard_value: 1.0,
            standard_radius: 1.0,
            colored_hue: 1.0,
            colored_saturation: 1.0,
            colored_value: 1.0,
            colored_radius: 1.0,
            duration_mult: 1.0,
            ..LightConfig::default()
        }
    }

    fn test_metadata(changed_cells: usize, changed_lights: usize) -> RunMetadata {
        RunMetadata {
            version: "test-version",
            config_path: PathBuf::from("/tmp/openmw.cfg"),
            output_path: PathBuf::from("/tmp/out/S3LightFixes.omwaddon"),
            content_files: 3,
            loaded_plugins: 2,
            masters: 1,
            changed_cells,
            changed_lights,
        }
    }

    #[test]
    fn generated_completion_goes_to_stdout_without_running_lightfixes() {
        let args = LightArgs::parse_from(["s3lightfixes", "--generate-completion", "bash"]);
        let mut stdout = Vec::new();

        assert!(handle_generated_output(&args, &mut stdout).unwrap());

        let completion = String::from_utf8(stdout).unwrap();
        assert!(completion.contains("_s3lightfixes"));
        assert!(completion.contains("--generate-manpage"));
    }

    #[test]
    fn generated_manpage_goes_to_stdout_without_running_lightfixes() {
        let args = LightArgs::parse_from(["s3lightfixes", "--generate-manpage"]);
        let mut stdout = Vec::new();

        assert!(handle_generated_output(&args, &mut stdout).unwrap());

        let manpage = String::from_utf8(stdout).unwrap();
        assert!(manpage.contains("s3lightfixes"));
        assert!(manpage.contains("A tool for modifying light values globally"));
    }

    #[test]
    fn generated_outputs_conflict_with_each_other() {
        let err = LightArgs::try_parse_from([
            "s3lightfixes",
            "--generate-completion",
            "bash",
            "--generate-manpage",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn dry_run_and_validate_config_conflict_with_each_other() {
        let err = LightArgs::try_parse_from(["s3lightfixes", "--dry-run", "--validate-config"])
            .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn dry_run_output_reports_target_and_planned_record_changes() {
        let mut stdout = Vec::new();
        let logs = [RecordLog {
            kind: "LIGH",
            plugin: "source.esp".to_owned(),
            id: "torch_01".to_owned(),
            changes: vec!["radius 10 -> 20".to_owned()],
        }];
        let metadata = test_metadata(0, 1);

        write_dry_run_to(&mut stdout, &metadata, &logs).unwrap();

        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Dry run: no files written"));
        assert!(output.contains("# output: /tmp/out/S3LightFixes.omwaddon"));
        assert!(output.contains("# changed lights: 1"));
        assert!(output.contains("LIGH \"torch_01\" from \"source.esp\": radius 10 -> 20"));
    }

    #[test]
    fn backup_openmw_cfg_copies_existing_user_config_before_auto_enable() {
        let temp_dir = std::env::temp_dir().join(format!(
            "s3lightfixes-openmw-backup-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&temp_dir).unwrap();
        let selected_config = temp_dir.join("custom-openmw.cfg");
        std::fs::write(&selected_config, "content=Morrowind.esm\n").unwrap();

        let backup_path = backup_openmw_cfg(&selected_config).unwrap();

        assert_eq!(
            backup_path,
            temp_dir.join("custom-openmw.cfg.s3lightfixes.bak")
        );
        assert_eq!(
            std::fs::read_to_string(backup_path).unwrap(),
            "content=Morrowind.esm\n"
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    fn interior_cell(id: &str) -> Cell {
        Cell {
            name: id.to_owned(),
            data: CellData {
                flags: CellFlags::IS_INTERIOR,
                ..CellData::default()
            },
            atmosphere_data: Some(AtmosphereData {
                ambient_color: [1, 2, 3, 0],
                sunlight_color: [4, 5, 6, 0],
                fog_color: [7, 8, 9, 0],
                fog_density: 0.25,
            }),
            water_height: Some(10.0),
            references: [((0, 1), Reference::default())].into(),
            ..Cell::default()
        }
    }

    fn exterior_cell(id: &str) -> Cell {
        Cell {
            name: id.to_owned(),
            atmosphere_data: Some(AtmosphereData::default()),
            water_height: Some(10.0),
            references: [((0, 1), Reference::default())].into(),
            ..Cell::default()
        }
    }

    #[test]
    fn generate_plugin_uses_first_processed_duplicate_id_and_keeps_unique_lights() {
        let later_path = temp_plugin_file("later.omwaddon", 11);
        let earlier_path = temp_plugin_file("earlier.omwaddon", 13);
        let light_config = config();
        let plugins = vec![
            (
                plugin_with_lights([light("Shared_Light", 300), light("later_only", 301)]),
                later_path.as_path(),
            ),
            (
                plugin_with_lights([light("shared_light", 100), light("earlier_only", 101)]),
                earlier_path.as_path(),
            ),
        ];

        let result = generate_plugin(plugins, &light_config).unwrap();
        let lights = generated_lights(&result.plugin);

        assert_eq!(lights.len(), 3);
        assert_eq!(
            lights
                .iter()
                .find(|light| light.id.eq_ignore_ascii_case("shared_light"))
                .unwrap()
                .data
                .radius,
            300
        );
        assert!(lights.iter().any(|light| light.id == "later_only"));
        assert!(lights.iter().any(|light| light.id == "earlier_only"));
        assert_eq!(result.header.num_objects, 3);
    }

    #[test]
    fn generate_plugin_adds_masters_only_for_plugins_that_contribute_objects() {
        let contributing_a = temp_plugin_file("contributing_a.omwaddon", 11);
        let duplicate_only = temp_plugin_file("duplicate_only.omwaddon", 13);
        let contributing_c = temp_plugin_file("contributing_c.omwaddon", 17);
        let light_config = config();
        let plugins = vec![
            (
                plugin_with_lights([light("shared", 100)]),
                contributing_a.as_path(),
            ),
            (
                plugin_with_lights([light("shared", 200)]),
                duplicate_only.as_path(),
            ),
            (
                plugin_with_lights([light("unique", 300)]),
                contributing_c.as_path(),
            ),
        ];

        let result = generate_plugin(plugins, &light_config).unwrap();

        assert_eq!(result.header.num_objects, 2);
        assert_eq!(
            result.header.masters,
            vec![
                (
                    contributing_c
                        .as_path()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    17
                ),
                (
                    contributing_a
                        .as_path()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    11
                ),
            ]
        );
    }

    #[test]
    fn compatibility_fixture_preserves_core_generation_contracts() {
        let first_processed = temp_plugin_file("first_processed.esp", 11);
        let second_processed = temp_plugin_file("second_processed.esp", 13);
        let mut light_config = config();
        light_config
            .excluded_id_regexes
            .push(Regex::new("excluded").unwrap());
        let plugins = vec![
            (
                plugin_with_lights([
                    light("duplicate_light", 10),
                    Light {
                        id: "negative_light".to_owned(),
                        data: LightData {
                            radius: 20,
                            color: [255, 128, 0, 0],
                            flags: LightFlags::NEGATIVE,
                            ..LightData::default()
                        },
                        ..Light::default()
                    },
                    light("excluded_light", 30),
                ]),
                first_processed.as_path(),
            ),
            (
                plugin_with_lights([
                    light("duplicate_light", 99),
                    light("second_unique_light", 40),
                ]),
                second_processed.as_path(),
            ),
        ];

        let mut result = generate_plugin(plugins, &light_config).unwrap();
        result
            .plugin
            .objects
            .push(TES3Object::Header(result.header));
        result.plugin.sort_objects();

        let generated = generated_lights(&result.plugin);
        assert_eq!(generated.len(), 3);
        assert!(
            generated
                .iter()
                .any(|light| light.id == "duplicate_light" && light.data.radius == 10)
        );
        assert!(generated.iter().all(|light| light.id != "excluded_light"));
        assert!(
            generated
                .iter()
                .any(|light| light.id == "second_unique_light" && light.data.radius == 40)
        );
        let negative = generated
            .iter()
            .find(|light| light.id == "negative_light")
            .unwrap();
        assert_eq!(negative.data.radius, 0);
        assert!(!negative.data.flags.contains(LightFlags::NEGATIVE));

        let TES3Object::Header(header) = &result.plugin.objects[0] else {
            panic!("generated plugin header was not sorted first");
        };
        assert_eq!(header.num_objects, 3);
        assert_eq!(header.masters.len(), 2);
        assert_eq!(result.logs.len(), 1);
        assert_eq!(result.logs[0].id, "negative_light");
    }

    #[test]
    fn process_lights_skips_excluded_ids_that_would_otherwise_emit() {
        let mut light_config = config();
        light_config
            .excluded_id_regexes
            .push(Regex::new("excluded_light").unwrap());
        let source_plugin = plugin_with_lights([light("excluded_light", 100), light("kept", 200)]);
        let mut generated_plugin = Plugin::new();
        let mut used_ids = HashSet::new();
        let mut logs = Vec::new();

        let used_objects = process_lights(
            source_plugin,
            "TestPlugin.esp",
            &mut generated_plugin,
            &light_config,
            &mut used_ids,
            &mut logs,
        );

        let lights = generated_lights(&generated_plugin);
        assert_eq!(used_objects, 1);
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].id, "kept");
        assert!(!used_ids.contains("excluded_light"));
        assert!(used_ids.contains("kept"));
        assert!(logs.is_empty());
    }

    #[test]
    fn process_lights_logs_actual_deltas_for_modified_lights() {
        let mut light_config = config();
        light_config.standard_radius = 2.0;
        let source_plugin = plugin_with_lights([light("modified_light", 100)]);
        let mut generated_plugin = Plugin::new();
        let mut used_ids = HashSet::new();
        let mut logs = Vec::new();

        let used_objects = process_lights(
            source_plugin,
            "ModifiedPlugin.esp",
            &mut generated_plugin,
            &light_config,
            &mut used_ids,
            &mut logs,
        );

        assert_eq!(used_objects, 1);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].kind, "LIGH");
        assert_eq!(logs[0].plugin, "ModifiedPlugin.esp");
        assert_eq!(logs[0].id, "modified_light");
        assert!(logs[0].changes.contains(&"radius 100 -> 200".to_owned()));
    }

    #[test]
    fn ambient_cell_replacement_consumes_id_before_light_processing() {
        let mut light_config = config();
        light_config.disable_interior_sun = true;
        let path = temp_plugin_file("shared_cell_light.omwaddon", 19);
        let plugin = Plugin {
            objects: vec![
                interior_cell("shared_id").into(),
                light("shared_id", 100).into(),
            ],
        };

        let result = generate_plugin(vec![(plugin, path.as_path())], &light_config).unwrap();

        assert_eq!(generated_cells(&result.plugin).len(), 1);
        assert!(generated_lights(&result.plugin).is_empty());
        assert_eq!(result.header.num_objects, 1);
    }

    #[test]
    fn process_cells_emits_ambient_replacement_and_strips_instance_state() {
        let mut light_config = config();
        light_config.ambient_regexes.push((
            Regex::new("ambient_cell").unwrap(),
            CustomCellAmbient {
                ambient: Some(TypedLightColor {
                    red: 0,
                    green: 255,
                    blue: 255,
                    migrated_from_hsv: false,
                }),
                sunlight: Some(TypedLightColor {
                    red: 0,
                    green: 0,
                    blue: 255,
                    migrated_from_hsv: false,
                }),
                fog: Some(TypedLightColor {
                    red: 0,
                    green: 255,
                    blue: 0,
                    migrated_from_hsv: false,
                }),
                fog_density: Some(0.75),
            },
        ));
        let mut source_plugin = Plugin {
            objects: vec![interior_cell("ambient_cell").into()],
        };
        let mut generated_plugin = Plugin::new();
        let mut used_ids = HashSet::new();
        let mut logs = Vec::new();

        let used_objects = process_cells(
            &mut source_plugin,
            "AmbientPlugin.esp",
            &mut generated_plugin,
            &light_config,
            &mut used_ids,
            &mut logs,
        );

        let cells = generated_cells(&generated_plugin);
        assert_eq!(used_objects, 1);
        assert_eq!(cells.len(), 1);
        assert!(used_ids.contains("ambient_cell"));
        assert!(cells[0].references.is_empty());
        assert!(cells[0].water_height.is_none());

        let atmo = cells[0].atmosphere_data.as_ref().unwrap();
        assert_eq!(atmo.ambient_color, [0, 255, 255, 0]);
        assert_eq!(atmo.sunlight_color, [0, 0, 255, 0]);
        assert_eq!(atmo.fog_color, [0, 255, 0, 0]);
        assert!((atmo.fog_density - 0.75).abs() < f32::EPSILON);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].kind, "CELL");
        assert_eq!(logs[0].plugin, "AmbientPlugin.esp");
        assert_eq!(logs[0].id, "ambient_cell");
        assert!(
            logs[0]
                .changes
                .contains(&"ambient [1, 2, 3, 0] -> [0, 255, 255, 0]".to_owned())
        );
        assert!(
            logs[0]
                .changes
                .contains(&"sunlight [4, 5, 6, 0] -> [0, 0, 255, 0]".to_owned())
        );
    }

    #[test]
    fn process_cells_disable_interior_sun_counts_as_replacement() {
        let mut light_config = config();
        light_config.disable_interior_sun = true;
        let mut source_plugin = Plugin {
            objects: vec![interior_cell("sun_cell").into()],
        };
        let mut generated_plugin = Plugin::new();
        let mut used_ids = HashSet::new();
        let mut logs = Vec::new();

        let used_objects = process_cells(
            &mut source_plugin,
            "SunPlugin.esp",
            &mut generated_plugin,
            &light_config,
            &mut used_ids,
            &mut logs,
        );

        let cells = generated_cells(&generated_plugin);
        assert_eq!(used_objects, 1);
        assert_eq!(cells.len(), 1);
        assert_eq!(
            cells[0].atmosphere_data.as_ref().unwrap().sunlight_color,
            [0, 0, 0, 0]
        );
        assert!(cells[0].references.is_empty());
        assert!(cells[0].water_height.is_none());
        assert!(used_ids.contains("sun_cell"));
        assert_eq!(logs.len(), 1);
        assert!(
            logs[0]
                .changes
                .contains(&"sunlight [4, 5, 6, 0] -> [0, 0, 0, 0]".to_owned())
        );
    }

    #[test]
    fn process_cells_does_not_log_stripped_patch_only_state() {
        let mut light_config = config();
        light_config.disable_interior_sun = true;
        let mut source_plugin = Plugin {
            objects: vec![
                Cell {
                    name: "already_dark_cell".to_owned(),
                    data: CellData {
                        flags: CellFlags::IS_INTERIOR,
                        ..CellData::default()
                    },
                    atmosphere_data: Some(AtmosphereData {
                        sunlight_color: [0, 0, 0, 0],
                        ..AtmosphereData::default()
                    }),
                    references: [((0, 1), Reference::default())].into(),
                    water_height: Some(42.0),
                    ..Cell::default()
                }
                .into(),
            ],
        };
        let mut generated_plugin = Plugin::new();
        let mut used_ids = HashSet::new();
        let mut logs = Vec::new();

        let used_objects = process_cells(
            &mut source_plugin,
            "AlreadyDark.esp",
            &mut generated_plugin,
            &light_config,
            &mut used_ids,
            &mut logs,
        );

        assert_eq!(used_objects, 1);
        assert!(logs.is_empty());
        assert!(generated_cells(&generated_plugin)[0].references.is_empty());
        assert!(generated_cells(&generated_plugin)[0].water_height.is_none());
    }

    #[test]
    fn process_cells_leaves_skipped_cells_out_of_generated_plugin() {
        let mut light_config = config();
        light_config.disable_interior_sun = true;
        light_config
            .excluded_id_regexes
            .push(Regex::new("excluded_cell").unwrap());
        let mut used_ids = HashSet::from(["duplicate_cell".to_owned()]);
        let mut source_plugin = Plugin {
            objects: vec![
                exterior_cell("exterior_cell").into(),
                Cell {
                    name: "no_atmosphere".to_owned(),
                    data: CellData {
                        flags: CellFlags::IS_INTERIOR,
                        ..CellData::default()
                    },
                    atmosphere_data: None,
                    ..Cell::default()
                }
                .into(),
                interior_cell("excluded_cell").into(),
                interior_cell("duplicate_cell").into(),
            ],
        };
        let mut generated_plugin = Plugin::new();
        let mut logs = Vec::new();

        let used_objects = process_cells(
            &mut source_plugin,
            "SkippedPlugin.esp",
            &mut generated_plugin,
            &light_config,
            &mut used_ids,
            &mut logs,
        );

        assert_eq!(used_objects, 0);
        assert!(generated_cells(&generated_plugin).is_empty());
        assert!(used_ids.contains("duplicate_cell"));
        assert!(!used_ids.contains("excluded_cell"));
        assert!(logs.is_empty());
    }

    #[test]
    fn write_log_reports_writer_errors() {
        struct BrokenWriter {
            attempted_write: bool,
        }

        impl Write for BrokenWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                self.attempted_write = true;
                Err(io::Error::other("broken writer"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut writer = BrokenWriter {
            attempted_write: false,
        };
        let logs = [RecordLog {
            kind: "LIGH",
            plugin: "BrokenPlugin.esp".to_owned(),
            id: "broken_writer".to_owned(),
            changes: vec!["radius 1 -> 2".to_owned()],
        }];

        let metadata = test_metadata(0, 1);

        let err = write_log_to(&mut writer, &metadata, &logs).unwrap_err();

        assert!(writer.attempted_write);
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn write_log_emits_one_line_per_modified_record() {
        let logs = [
            RecordLog {
                kind: "CELL",
                plugin: "Morrowind.esm".to_owned(),
                id: "cell_id".to_owned(),
                changes: vec![
                    "sunlight [1, 2, 3, 0] -> [0, 0, 0, 0]".to_owned(),
                    "fog_density 0.5 -> 0.75".to_owned(),
                ],
            },
            RecordLog {
                kind: "LIGH",
                plugin: "Tribunal.esm".to_owned(),
                id: "light_id".to_owned(),
                changes: vec![
                    "color [1, 2, 3, 0] -> [4, 5, 6, 0]".to_owned(),
                    "radius 128 -> 256".to_owned(),
                ],
            },
        ];
        let mut output = Vec::new();
        let metadata = test_metadata(1, 1);

        write_log_to(&mut output, &metadata, &logs).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("# S3LightFixes test-version"));
        assert!(output.contains("# content files: 3"));
        assert!(output.contains("# loaded plugins: 2"));
        assert!(output.contains("# changed cells: 1"));
        assert!(output.contains("# changed lights: 1"));
        assert!(output.contains("CELL \"cell_id\" from \"Morrowind.esm\": sunlight [1, 2, 3, 0] -> [0, 0, 0, 0], fog_density 0.5 -> 0.75"));
        assert!(output.contains("LIGH \"light_id\" from \"Tribunal.esm\": color [1, 2, 3, 0] -> [4, 5, 6, 0], radius 128 -> 256"));
    }
}
