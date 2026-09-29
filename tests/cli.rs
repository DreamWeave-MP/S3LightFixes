use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn scratch_dir(name: &str) -> PathBuf {
    let directory =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn run_quietly(arguments: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_s3lightfixes"))
        .args(arguments)
        .env("S3L_NO_NOTIFICATIONS", "1")
        .output()
        .unwrap()
}

#[test]
fn unreadable_lightconfig_fails_with_a_nonzero_status() {
    let directory = scratch_dir("unreadable-lightconfig");
    std::fs::write(directory.join("openmw.cfg"), "content=Morrowind.esm\n").unwrap();
    std::fs::write(
        directory.join("lightconfig.toml"),
        "standard_hue = \"oops\"\n",
    )
    .unwrap();

    let output = run_quietly(&[
        "--openmw-cfg".as_ref(),
        directory.as_os_str(),
        "--validate-config".as_ref(),
    ]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Lightconfig.toml couldn't be read"),
        "{output:?}"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[cfg(target_os = "linux")]
#[test]
fn notifications_print_when_no_dialog_program_can_be_found() {
    let directory = scratch_dir("no-dialog-program");
    let missing = directory.join("missing");

    let output = Command::new(env!("CARGO_BIN_EXE_s3lightfixes"))
        .arg("--openmw-cfg")
        .arg(&missing)
        .env("PATH", "")
        .env_remove("S3L_NO_NOTIFICATIONS")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(127), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("could not be resolved"),
        "{output:?}"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn validate_config_fails_on_unknown_keys_and_names_them() {
    let directory = scratch_dir("unknown-lightconfig-keys");
    std::fs::write(directory.join("openmw.cfg"), "content=Morrowind.esm\n").unwrap();
    std::fs::write(
        directory.join("lightconfig.toml"),
        "standard_hue = 0.5\nstandard_satuation = 0.7\n\n[light_overrides.\"^torch\"]\nraduis = 300\n",
    )
    .unwrap();

    let output = run_quietly(&[
        "--openmw-cfg".as_ref(),
        directory.as_os_str(),
        "--validate-config".as_ref(),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(stdout.contains("`standard_satuation`"), "{output:?}");
    assert!(
        stdout.contains("`light_overrides.\"^torch\".raduis`"),
        "{output:?}"
    );
    assert!(!stdout.contains("successfully"), "{output:?}");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn validate_config_accepts_keys_older_versions_wrote() {
    let directory = scratch_dir("retired-lightconfig-keys");
    std::fs::write(directory.join("openmw.cfg"), "content=Morrowind.esm\n").unwrap();
    std::fs::write(
        directory.join("lightconfig.toml"),
        "auto_install = true\nsave_log = false\nstandard_hue = 0.5\n",
    )
    .unwrap();

    let output = run_quietly(&[
        "--openmw-cfg".as_ref(),
        directory.as_os_str(),
        "--validate-config".as_ref(),
    ]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        stdout.contains("`save_log` is no longer used"),
        "{output:?}"
    );
    assert!(
        stdout.contains("`auto_install` is no longer used"),
        "{output:?}"
    );
    assert!(stdout.contains("successfully"), "{output:?}");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_run_warns_about_unknown_keys_and_goes_on() {
    let directory = scratch_dir("unknown-lightconfig-keys-run");
    std::fs::write(directory.join("openmw.cfg"), "content=Morrowind.esm\n").unwrap();
    std::fs::write(
        directory.join("lightconfig.toml"),
        "standard_satuation = 0.7\nsave_log = false\n",
    )
    .unwrap();

    let output = run_quietly(&[
        "--openmw-cfg".as_ref(),
        directory.as_os_str(),
        "--dry-run".as_ref(),
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stderr.contains("`standard_satuation`"), "{output:?}");
    assert!(!stderr.contains("save_log"), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Dry run: no files written"),
        "{output:?}"
    );

    let _ = std::fs::remove_dir_all(directory);
}
