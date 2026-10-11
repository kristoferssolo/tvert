use claims::assert_ok;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn check_config(path: &Path) -> Output {
    assert_ok!(
        Command::new(env!("CARGO_BIN_EXE_tvert"))
            .arg("check-config")
            .arg("--config")
            .arg(path)
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("WAYLAND_SOCKET")
            .env_remove("XDG_RUNTIME_DIR")
            .output()
    )
}

#[test]
fn check_config_validates_the_requested_file_withtout_wayland() {
    let directory = assert_ok!(tempdir());
    let path = directory.path().join("config.toml");

    assert_ok!(fs::write(&path, "[editor]\ntool = 'arrows'\n"));
    let invalid = check_config(&path);

    assert_eq!(invalid.status.code(), Some(1));
    assert_eq!(invalid.stdout.as_slice(), b"");

    let diagnostic = assert_ok!(String::from_utf8(invalid.stderr));
    assert!(diagnostic.contains("unknown variant `arrows`"));

    assert_ok!(fs::write(&path, "[editor]\nstroke_width = 7\n"));
    let valid = check_config(&path);

    assert!(valid.status.success());
    assert_eq!(valid.stderr.as_slice(), b"");
}
