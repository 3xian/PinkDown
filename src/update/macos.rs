//! macOS install: mount the release DMG and replace `PinkDown.app` after exit.

#![cfg(any(test, target_os = "macos"))]

use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

#[cfg(target_os = "macos")]
use super::{updater_paths, wait_for_updater_ready, UpdateError};

#[cfg(not(target_os = "macos"))]
use super::UpdateError;

#[cfg(target_os = "macos")]
pub(super) fn schedule(downloaded: &Path) -> Result<(), UpdateError> {
    let app_bundle = app_bundle()?;
    ensure_bundle_parent_writable(&app_bundle)?;

    let paths = updater_paths("sh");
    let _ = fs::remove_file(&paths.ready);

    let script_text = update_script(
        std::process::id(),
        downloaded,
        &app_bundle,
        &paths.ready,
        &paths.log,
        &paths.script,
    );
    fs::write(&paths.script, script_text)
        .map_err(|error| UpdateError::new(format!("Could not prepare updater: {error}")))?;
    fs::set_permissions(&paths.script, fs::Permissions::from_mode(0o700))
        .map_err(|error| UpdateError::new(format!("Could not prepare updater: {error}")))?;

    let mut child = Command::new("/bin/bash")
        .arg(&paths.script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| UpdateError::new(format!("Could not start updater: {error}")))?;

    wait_for_updater_ready(&mut child, &paths.ready, &paths.script, &paths.log)
}

#[cfg(target_os = "macos")]
fn app_bundle() -> Result<PathBuf, UpdateError> {
    let current = std::env::current_exe()
        .map_err(|error| UpdateError::new(format!("Could not locate the running app: {error}")))?;
    let current = current.canonicalize().unwrap_or(current);
    app_bundle_from(&current)
}

/// Resolves the `.app` bundle that contains the running executable.
pub(super) fn app_bundle_from(current: &Path) -> Result<PathBuf, UpdateError> {
    let mut path = current.to_path_buf();
    for _ in 0..6 {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".app"))
        {
            return Ok(path);
        }
        path = path
            .parent()
            .ok_or_else(|| {
                UpdateError::new(
                    "Automatic updates require PinkDown.app (install from the DMG first)",
                )
            })?
            .to_path_buf();
    }
    Err(UpdateError::new(
        "Automatic updates require PinkDown.app (install from the DMG first)",
    ))
}

/// Fail before handoff when the install location is not writable.
#[cfg(target_os = "macos")]
fn ensure_bundle_parent_writable(app_bundle: &Path) -> Result<(), UpdateError> {
    let parent = app_bundle.parent().ok_or_else(|| {
        UpdateError::new("Could not determine the install directory for PinkDown.app")
    })?;
    let probe = parent.join(format!(".pinkdown-update-write-test-{}", std::process::id()));
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(error) => Err(UpdateError::new(format!(
            "Cannot write to {}: {error}",
            parent.display()
        ))),
    }
}

/// Bash helper: wait for exit, mount DMG, stage then replace the `.app`, clear
/// quarantine, relaunch.
pub(super) fn update_script(
    parent_id: u32,
    dmg: &Path,
    app_bundle: &Path,
    ready: &Path,
    log: &Path,
    script: &Path,
) -> String {
    let quote = |path: &Path| format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
    [
        "#!/bin/bash".to_owned(),
        "set -u".to_owned(),
        format!("parent_id={parent_id}"),
        format!("dmg={}", quote(dmg)),
        format!("app_bundle={}", quote(app_bundle)),
        format!("ready={}", quote(ready)),
        format!("log={}", quote(log)),
        format!("script={}", quote(script)),
        "mountpoint=\"\"".to_owned(),
        "staging=\"\"".to_owned(),
        "cleanup() {".to_owned(),
        "  if [ -n \"${mountpoint}\" ] && [ -d \"${mountpoint}\" ]; then".to_owned(),
        "    hdiutil detach \"${mountpoint}\" -force >/dev/null 2>&1 || true".to_owned(),
        "    rmdir \"${mountpoint}\" >/dev/null 2>&1 || true".to_owned(),
        "  fi".to_owned(),
        "  if [ -n \"${staging}\" ] && [ -e \"${staging}\" ]; then".to_owned(),
        "    rm -rf \"${staging}\"".to_owned(),
        "  fi".to_owned(),
        "  rm -f \"${ready}\" \"${script}\"".to_owned(),
        "}".to_owned(),
        "trap cleanup EXIT".to_owned(),
        "printf 'ready\\n' > \"${ready}\"".to_owned(),
        "if kill -0 \"${parent_id}\" 2>/dev/null; then".to_owned(),
        "  while kill -0 \"${parent_id}\" 2>/dev/null; do sleep 0.25; done".to_owned(),
        "fi".to_owned(),
        "sleep 0.5".to_owned(),
        "{".to_owned(),
        "  mountpoint=$(mktemp -d \"${TMPDIR:-/tmp}/pinkdown-dmg.XXXXXX\")".to_owned(),
        "  hdiutil attach \"${dmg}\" -nobrowse -readonly -mountpoint \"${mountpoint}\"".to_owned(),
        "  src=\"${mountpoint}/PinkDown.app\"".to_owned(),
        "  if [ ! -d \"${src}\" ]; then".to_owned(),
        "    echo 'PinkDown.app not found in disk image' >&2".to_owned(),
        "    exit 1".to_owned(),
        "  fi".to_owned(),
        // Stage beside the live bundle, then swap so a crash mid-copy cannot
        // leave a half-written PinkDown.app.
        "  staging=\"${app_bundle}.new\"".to_owned(),
        "  rm -rf \"${staging}\"".to_owned(),
        "  ditto \"${src}\" \"${staging}\"".to_owned(),
        "  chmod +x \"${staging}/Contents/MacOS/pinkdown\"".to_owned(),
        "  hdiutil detach \"${mountpoint}\"".to_owned(),
        "  rmdir \"${mountpoint}\" >/dev/null 2>&1 || true".to_owned(),
        "  mountpoint=\"\"".to_owned(),
        "  backup=\"${app_bundle}.old\"".to_owned(),
        "  rm -rf \"${backup}\"".to_owned(),
        "  if [ -e \"${app_bundle}\" ]; then mv \"${app_bundle}\" \"${backup}\"; fi".to_owned(),
        "  mv \"${staging}\" \"${app_bundle}\"".to_owned(),
        "  staging=\"\"".to_owned(),
        "  rm -rf \"${backup}\"".to_owned(),
        "  xattr -cr \"${app_bundle}\" 2>/dev/null || true".to_owned(),
        "  open \"${app_bundle}\"".to_owned(),
        "  rm -f \"${dmg}\"".to_owned(),
        "  rm -f \"${log}\"".to_owned(),
        "} >\"${log}\" 2>&1 || {".to_owned(),
        "  if [ -d \"${app_bundle}\" ]; then open \"${app_bundle}\" || true; fi".to_owned(),
        "  if [ ! -d \"${app_bundle}\" ] && [ -d \"${app_bundle}.old\" ]; then".to_owned(),
        "    mv \"${app_bundle}.old\" \"${app_bundle}\" 2>/dev/null || true".to_owned(),
        "    open \"${app_bundle}\" || true".to_owned(),
        "  fi".to_owned(),
        "  exit 1".to_owned(),
        "}".to_owned(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{app_bundle_from, update_script};
    use std::path::{Path, PathBuf};

    #[test]
    fn installer_script_stages_then_replaces_the_app_bundle() {
        let script = update_script(
            42,
            Path::new("pinkdown.dmg"),
            Path::new("/Applications/PinkDown.app"),
            Path::new("ready"),
            Path::new("error.log"),
            Path::new("update.sh"),
        );
        assert!(script.contains("#!/bin/bash"));
        assert!(script.contains("kill -0 \"${parent_id}\""));
        assert!(script.contains("hdiutil attach \"${dmg}\""));
        assert!(script.contains("staging=\"${app_bundle}.new\""));
        assert!(script.contains("ditto \"${src}\" \"${staging}\""));
        assert!(script.contains("mv \"${staging}\" \"${app_bundle}\""));
        assert!(script.contains("xattr -cr \"${app_bundle}\""));
        assert!(script.contains("open \"${app_bundle}\""));
        assert!(script.contains("rm -f \"${dmg}\""));
    }

    #[test]
    fn app_bundle_walks_up_from_the_executable() {
        let exe = PathBuf::from("/Applications/PinkDown.app/Contents/MacOS/pinkdown");
        let bundle = app_bundle_from(&exe).unwrap();
        assert_eq!(bundle, PathBuf::from("/Applications/PinkDown.app"));
    }

    #[test]
    fn app_bundle_rejects_unpackaged_binaries() {
        let error = app_bundle_from(&PathBuf::from("/tmp/pinkdown")).unwrap_err();
        assert!(error.to_string().contains("PinkDown.app"));
    }
}
