//! Windows silent install via Inno Setup after PinkDown exits.

#![cfg(any(test, target_os = "windows"))]

#[cfg(target_os = "windows")]
use std::{
    fs,
    os::windows::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
use super::{updater_paths, wait_for_updater_ready, UpdateError};

#[cfg(target_os = "windows")]
pub(super) fn schedule(downloaded: &Path) -> Result<(), UpdateError> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let current = std::env::current_exe()
        .map_err(|error| UpdateError::new(format!("Could not locate the running app: {error}")))?;
    let paths = updater_paths("ps1");
    let _ = fs::remove_file(&paths.ready);

    let script_text = update_script(
        std::process::id(),
        downloaded,
        &current,
        &paths.ready,
        &paths.log,
        &paths.script,
    );
    fs::write(&paths.script, script_text)
        .map_err(|error| UpdateError::new(format!("Could not prepare updater: {error}")))?;

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&paths.script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| UpdateError::new(format!("Could not start updater: {error}")))?;

    wait_for_updater_ready(&mut child, &paths.ready, &paths.script, &paths.log)
}

pub(super) fn update_script(
    parent_id: u32,
    installer: &std::path::Path,
    current: &std::path::Path,
    ready: &std::path::Path,
    log: &std::path::Path,
    script: &std::path::Path,
) -> String {
    let quoted =
        |path: &std::path::Path| format!("'{}'", path.display().to_string().replace('\'', "''"));
    [
        "$ErrorActionPreference = 'Stop'".to_owned(),
        format!("$parentId = {parent_id}"),
        format!("$installer = {}", quoted(installer)),
        format!("$current = {}", quoted(current)),
        "$installDir = Split-Path -Parent $current".to_owned(),
        format!("$ready = {}", quoted(ready)),
        format!("$log = {}", quoted(log)),
        format!("$script = {}", quoted(script)),
        "try {".to_owned(),
        "    Set-Content -LiteralPath $ready -Value 'ready' -Encoding ascii".to_owned(),
        "    $parent = Get-Process -Id $parentId -ErrorAction SilentlyContinue".to_owned(),
        "    if ($parent) { Wait-Process -Id $parentId }".to_owned(),
        "    $installArgs = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/DIR=\"' + $installDir + '\"'))".to_owned(),
        "    $setup = Start-Process -FilePath $installer -ArgumentList $installArgs -Wait -PassThru"
            .to_owned(),
        "    if ($setup.ExitCode -ne 0) { throw \"Installer exited with code $($setup.ExitCode)\" }"
            .to_owned(),
        "    if (!(Test-Path -LiteralPath $current)) { throw 'Installed application was not found' }"
            .to_owned(),
        "    Start-Process -FilePath $current".to_owned(),
        "    Remove-Item -LiteralPath $installer -Force".to_owned(),
        "    if (Test-Path -LiteralPath $log) { Remove-Item -LiteralPath $log -Force }".to_owned(),
        "} catch {".to_owned(),
        "    ($_ | Out-String) | Set-Content -LiteralPath $log".to_owned(),
        "    if (Test-Path -LiteralPath $current) { Start-Process -FilePath $current }"
            .to_owned(),
        "} finally {".to_owned(),
        "    if (Test-Path -LiteralPath $ready) { Remove-Item -LiteralPath $ready -Force }".to_owned(),
        "    Remove-Item -LiteralPath $script -Force".to_owned(),
        "}".to_owned(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::update_script;
    use std::path::Path;

    #[test]
    fn installer_script_waits_then_runs_setup_in_the_existing_directory() {
        let script = update_script(
            42,
            Path::new("pinkdown-setup.exe"),
            Path::new("current.exe"),
            Path::new("ready"),
            Path::new("error.log"),
            Path::new("update.ps1"),
        );
        assert!(script.contains("Set-Content -LiteralPath $ready"));
        assert!(script.contains("Wait-Process -Id $parentId"));
        assert!(script.contains("Start-Process -FilePath $installer"));
        assert!(script.contains("'/DIR=\"' + $installDir + '\"'"));
        assert!(script.contains("if ($setup.ExitCode -ne 0)"));
        assert!(script.contains("Start-Process -FilePath $current"));
        assert!(script.contains("Set-Content -LiteralPath $log"));
    }
}
