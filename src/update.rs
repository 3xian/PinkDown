use std::{
    fmt,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use semver::Version;
use serde::Deserialize;

const GITHUB_TAGS_URL: &str = "https://api.github.com/repos/3xian/PinkDown/tags?per_page=100";

#[cfg(target_os = "windows")]
const GITHUB_RELEASES_URL: &str = "https://github.com/3xian/PinkDown/releases/download";
#[cfg(target_os = "windows")]
const WINDOWS_RELEASE_ASSET: &str = "pinkdown-windows-x64-setup.exe";

#[derive(Debug)]
pub struct UpdateError(String);

impl UpdateError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub enum UpdateOutcome {
    UpToDate(Version),
    #[cfg(target_os = "windows")]
    InstallReady(Version),
    #[cfg(not(target_os = "windows"))]
    ManualUpdate(Version),
}

pub enum PollResult {
    Idle,
    Pending,
    Ready(Result<UpdateOutcome, UpdateError>),
}

#[derive(Default)]
pub struct UpdateChecker {
    receiver: Option<Receiver<Result<UpdateOutcome, UpdateError>>>,
}

impl UpdateChecker {
    pub fn start(&mut self) -> bool {
        if self.receiver.is_some() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        thread::spawn(move || {
            let _ = sender.send(check_for_update());
        });
        true
    }

    pub fn poll(&mut self) -> PollResult {
        let Some(receiver) = &self.receiver else {
            return PollResult::Idle;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                PollResult::Ready(result)
            }
            Err(TryRecvError::Empty) => PollResult::Pending,
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                PollResult::Ready(Err(UpdateError::new("Update check did not complete")))
            }
        }
    }
}

#[derive(Deserialize)]
struct GitHubTag {
    name: String,
}

fn check_for_update() -> Result<UpdateOutcome, UpdateError> {
    #[cfg(target_os = "windows")]
    let (latest_tag, latest_version) = latest_github_tag()?;
    #[cfg(not(target_os = "windows"))]
    let (_, latest_version) = latest_github_tag()?;
    let current_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| UpdateError::new(format!("Invalid current version: {error}")))?;

    if latest_version <= current_version {
        return Ok(UpdateOutcome::UpToDate(current_version));
    }

    #[cfg(target_os = "windows")]
    {
        let downloaded = download_windows_update(&latest_tag)?;
        if let Err(error) = schedule_windows_update(&downloaded) {
            let _ = std::fs::remove_file(downloaded);
            return Err(error);
        }
        Ok(UpdateOutcome::InstallReady(latest_version))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(UpdateOutcome::ManualUpdate(latest_version))
    }
}

fn latest_github_tag() -> Result<(String, Version), UpdateError> {
    let tags: Vec<GitHubTag> = ureq::get(GITHUB_TAGS_URL)
        .set(
            "User-Agent",
            concat!("PinkDown/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| UpdateError::new(format!("Could not contact GitHub: {error}")))?
        .into_json()
        .map_err(|error| UpdateError::new(format!("Could not read GitHub tags: {error}")))?;
    select_latest_tag(tags.into_iter().map(|tag| tag.name))
}

fn select_latest_tag(
    tags: impl IntoIterator<Item = String>,
) -> Result<(String, Version), UpdateError> {
    tags.into_iter()
        .filter_map(|tag| version_from_tag(&tag).ok().map(|version| (tag, version)))
        .max_by(|left, right| left.1.cmp(&right.1))
        .ok_or_else(|| UpdateError::new("No semantic-version tags found on GitHub"))
}

fn version_from_tag(tag: &str) -> Result<Version, semver::Error> {
    Version::parse(tag.trim_start_matches('v'))
}

#[cfg(target_os = "windows")]
fn download_windows_update(tag: &str) -> Result<std::path::PathBuf, UpdateError> {
    use std::{fs, io, io::Read};

    let asset_url = format!("{GITHUB_RELEASES_URL}/{tag}/{WINDOWS_RELEASE_ASSET}");
    let checksum_url = format!("{asset_url}.sha256");
    let expected_checksum = download_text(&checksum_url)?
        .split_whitespace()
        .next()
        .filter(|checksum| checksum.len() == 64 && checksum.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| UpdateError::new("Release checksum is missing or invalid"))?
        .to_ascii_lowercase();
    let destination =
        std::env::temp_dir().join(format!("pinkdown-{tag}-{}-setup.exe", std::process::id()));

    let result = (|| {
        let response = ureq::get(&asset_url)
            .set(
                "User-Agent",
                concat!("PinkDown/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .map_err(|error| UpdateError::new(format!("Could not download {tag}: {error}")))?;
        let mut source = response.into_reader().take(256 * 1024 * 1024 + 1);
        let mut file = fs::File::create(&destination)
            .map_err(|error| UpdateError::new(format!("Could not create update file: {error}")))?;
        let bytes_written = io::copy(&mut source, &mut file)
            .map_err(|error| UpdateError::new(format!("Could not save update: {error}")))?;
        if bytes_written > 256 * 1024 * 1024 {
            return Err(UpdateError::new(
                "Downloaded update exceeds the 256 MiB limit",
            ));
        }

        let actual_checksum = sha256_file(&destination)?;
        if actual_checksum != expected_checksum {
            return Err(UpdateError::new(
                "Downloaded update did not match its release checksum",
            ));
        }
        Ok(destination.clone())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&destination);
    }
    result
}

#[cfg(target_os = "windows")]
fn download_text(url: &str) -> Result<String, UpdateError> {
    use std::io::Read;

    let mut response = ureq::get(url)
        .set(
            "User-Agent",
            concat!("PinkDown/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| UpdateError::new(format!("Could not download checksum: {error}")))?
        .into_reader();
    let mut text = String::new();
    response
        .read_to_string(&mut text)
        .map_err(|error| UpdateError::new(format!("Could not read checksum: {error}")))?;
    Ok(text)
}

#[cfg(target_os = "windows")]
fn sha256_file(path: &std::path::Path) -> Result<String, UpdateError> {
    use std::{fs, io::Read};

    use sha2::{Digest, Sha256};

    let mut file = fs::File::open(path)
        .map_err(|error| UpdateError::new(format!("Could not verify update: {error}")))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 32 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|error| UpdateError::new(format!("Could not verify update: {error}")))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(target_os = "windows")]
fn schedule_windows_update(downloaded: &std::path::Path) -> Result<(), UpdateError> {
    use std::{
        fs,
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let current = std::env::current_exe()
        .map_err(|error| UpdateError::new(format!("Could not locate the running app: {error}")))?;
    let temp = std::env::temp_dir();
    let process_id = std::process::id();
    let script = temp.join(format!("pinkdown-update-{process_id}.ps1"));
    let ready = temp.join(format!("pinkdown-update-{process_id}.ready"));
    let log = temp.join(format!("pinkdown-update-{process_id}.log"));
    let _ = fs::remove_file(&ready);

    let script_text =
        windows_update_script(process_id, downloaded, &current, &ready, &log, &script);
    fs::write(&script, script_text)
        .map_err(|error| UpdateError::new(format!("Could not prepare updater: {error}")))?;

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| UpdateError::new(format!("Could not start updater: {error}")))?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if ready.is_file() {
            return Ok(());
        }
        if child
            .try_wait()
            .map_err(|error| UpdateError::new(format!("Could not inspect updater: {error}")))?
            .is_some()
        {
            return Err(UpdateError::new(format!(
                "Updater exited before handoff; see {}",
                log.display()
            )));
        }
        thread::sleep(Duration::from_millis(25));
    }

    let _ = child.kill();
    let _ = fs::remove_file(&script);
    Err(UpdateError::new("Updater did not acknowledge the handoff"))
}

#[cfg(target_os = "windows")]
fn windows_update_script(
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
    use super::*;

    #[test]
    fn selects_highest_semantic_version_and_ignores_other_tags() {
        let tags = ["nightly", "v1.9.0", "v2.0.0-beta.1", "v1.10.0"].map(str::to_owned);
        let (tag, version) = select_latest_tag(tags).unwrap();
        assert_eq!(tag, "v2.0.0-beta.1");
        assert_eq!(version, Version::parse("2.0.0-beta.1").unwrap());
    }

    #[test]
    fn rejects_a_tag_set_without_semantic_versions() {
        let error = select_latest_tag(["latest".to_owned()]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "No semantic-version tags found on GitHub"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn installer_script_waits_then_runs_setup_in_the_existing_directory() {
        use std::path::Path;

        let script = windows_update_script(
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
