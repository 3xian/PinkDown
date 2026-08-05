use std::{
    fmt,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use semver::Version;
use serde::Deserialize;

const GITHUB_TAGS_URL: &str = "https://api.github.com/repos/3xian/PinkDown/tags?per_page=100";
const PINKDOWN_USER_AGENT: &str = concat!("PinkDown/", env!("CARGO_PKG_VERSION"));

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
    let tags: Vec<GitHubTag> = github_get(GITHUB_TAGS_URL, github_token().as_deref())?
        .into_json()
        .map_err(|error| UpdateError::new(format!("Could not read GitHub tags: {error}")))?;
    select_latest_tag(tags.into_iter().map(|tag| tag.name))
}

/// Issues a GitHub API GET with PinkDown's user agent and, when a token is
/// supplied, bearer authentication. Rate-limited responses (403/429) are
/// translated into an actionable message instead of a bare status code.
fn github_get(url: &str, token: Option<&str>) -> Result<ureq::Response, UpdateError> {
    let request = ureq::get(url).set("User-Agent", PINKDOWN_USER_AGENT);
    let request = match token {
        Some(token) => request.set("Authorization", &format!("Bearer {token}")),
        None => request,
    };
    request.call().map_err(|error| match error {
        ureq::Error::Status(status, response) if status == 403 || status == 429 => {
            github_api_error(status, &response.into_string().unwrap_or_default())
        }
        error => UpdateError::new(format!("Could not contact GitHub: {error}")),
    })
}

/// The GitHub token used to authenticate API requests, read from
/// `GITHUB_TOKEN` or `GH_TOKEN`. Unauthenticated GitHub API requests are
/// limited to 60 per hour per IP; authenticated requests get 5,000 per hour.
fn github_token() -> Option<String> {
    github_token_from(|name| std::env::var(name))
}

fn github_token_from(
    var: impl Fn(&str) -> Result<String, std::env::VarError>,
) -> Option<String> {
    var("GITHUB_TOKEN")
        .or_else(|_| var("GH_TOKEN"))
        .ok()
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

fn github_api_error(status: u16, body: &str) -> UpdateError {
    if (status == 403 || status == 429) && body.contains("rate limit") {
        UpdateError::new(
            "GitHub API rate limit exceeded; set GITHUB_TOKEN or GH_TOKEN to a personal access token to raise it",
        )
    } else if body.trim().is_empty() {
        UpdateError::new(format!("GitHub API returned status {status}"))
    } else {
        UpdateError::new(format!("GitHub API returned status {status}: {body}"))
    }
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
            .set("User-Agent", PINKDOWN_USER_AGENT)
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
        .set("User-Agent", PINKDOWN_USER_AGENT)
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

    /// Sends a tags request to a local HTTP listener and returns the
    /// `Authorization` header it received, if any.
    fn received_authorization(token: Option<&str>) -> Option<String> {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/tags", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut received = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let bytes_read = stream.read(&mut buffer).unwrap();
                if bytes_read == 0 {
                    break;
                }
                received.extend_from_slice(&buffer[..bytes_read]);
                if received.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
            String::from_utf8_lossy(&received)
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
                .map(str::to_owned)
        });
        github_get(&url, token).unwrap();
        server.join().unwrap()
    }

    #[test]
    fn tags_request_attaches_the_token_as_bearer_auth() {
        let authorization = received_authorization(Some("secret-token"));
        assert_eq!(authorization.as_deref(), Some("Authorization: Bearer secret-token"));
    }

    #[test]
    fn tags_request_is_anonymous_without_a_token() {
        assert_eq!(received_authorization(None), None);
    }

    #[test]
    fn github_token_prefers_github_token_over_gh_token() {
        let token = github_token_from(|name| match name {
            "GITHUB_TOKEN" => Ok("primary".to_owned()),
            "GH_TOKEN" => Ok("secondary".to_owned()),
            _ => Err(std::env::VarError::NotPresent),
        });
        assert_eq!(token.as_deref(), Some("primary"));
    }

    #[test]
    fn github_token_falls_back_to_gh_token() {
        let token = github_token_from(|name| match name {
            "GH_TOKEN" => Ok("secondary".to_owned()),
            _ => Err(std::env::VarError::NotPresent),
        });
        assert_eq!(token.as_deref(), Some("secondary"));
    }

    #[test]
    fn github_token_ignores_blank_values() {
        assert_eq!(github_token_from(|_| Ok(String::new())), None);
        assert_eq!(github_token_from(|_| Ok("  ".to_owned())), None);
    }

    #[test]
    fn github_token_trims_whitespace_around_the_value() {
        assert_eq!(
            github_token_from(|_| Ok("  secret-token\n".to_owned())).as_deref(),
            Some("secret-token")
        );
    }

    #[test]
    fn github_token_is_none_without_env_vars() {
        assert_eq!(
            github_token_from(|_| Err(std::env::VarError::NotPresent)),
            None
        );
    }

    #[test]
    fn rate_limited_api_errors_explain_the_token_fix() {
        let error = github_api_error(
            403,
            r#"{"message":"API rate limit exceeded for 1.2.3.4"}"#,
        );
        assert!(error.to_string().contains("GITHUB_TOKEN"));
        assert!(github_api_error(429, "API rate limit exceeded")
            .to_string()
            .contains("GITHUB_TOKEN"));
    }

    #[test]
    fn non_rate_limit_api_errors_report_the_status() {
        let error = github_api_error(404, "Not Found");
        assert_eq!(
            error.to_string(),
            "GitHub API returned status 404: Not Found"
        );
    }

    #[test]
    fn api_errors_without_a_body_report_only_the_status() {
        assert_eq!(
            github_api_error(403, "").to_string(),
            "GitHub API returned status 403"
        );
    }

    #[test]
    fn windows_installer_registers_markdown_and_opens_default_apps_confirmation() {
        let installer = include_str!("../installer/pinkdown.iss");

        assert!(installer.contains("Software\\Classes\\.md\\OpenWithProgids"));
        assert!(installer.contains("Software\\RegisteredApplications"));
        assert!(installer.contains("registeredAppUser=PinkDown"));
        assert!(installer.contains("Tasks: associate_md"));
        assert!(installer.contains("{#MyAppExeName}\"\" \"\"%1"));
    }

    #[test]
    fn mac_release_packages_a_dmg_with_a_retina_icon() {
        let plist = include_str!("../installer/macos/Info.plist");
        let packager = include_str!("../installer/macos/package.ps1");
        let workflow = include_str!("../.github/workflows/release.yml");

        assert!(plist.contains("<key>CFBundlePackageType</key>"));
        assert!(plist.contains("<string>APPL</string>"));
        assert!(plist.contains("<key>CFBundleIconFile</key>"));
        assert!(plist.contains("<string>PinkDown.icns</string>"));
        assert!(packager.contains("icon_16x16.png"));
        assert!(packager.contains("icon_512x512@2x.png"));
        assert!(packager.contains("hdiutil create"));
        assert!(packager.contains("ln -s '/Applications'"));
        assert!(workflow.contains("pinkdown-macos-arm64.dmg"));
        assert!(workflow.contains("pinkdown-macos-x64.dmg"));
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
