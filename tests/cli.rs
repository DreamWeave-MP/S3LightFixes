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
