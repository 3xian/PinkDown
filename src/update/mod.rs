//! GitHub-based update check and platform installers.
//!
//! Flow: check tags → [`UpdateOutcome::Available`] → user confirms → download
//! + stage → [`UpdateOutcome::InstallReady`] → app quits → helper applies package.

mod macos;
mod windows;

use std::{
    fmt,
    path::{Path, PathBuf},
    process::Child,
    sync::{
        mpsc::{self, Receiver, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use semver::Version;
use serde::Deserialize;

const GITHUB_TAGS_URL: &str = "https://api.github.com/repos/3xian/PinkDown/tags?per_page=100";
const PINKDOWN_USER_AGENT: &str = concat!("PinkDown/", env!("CARGO_PKG_VERSION"));

#[cfg(any(target_os = "windows", target_os = "macos"))]
const GITHUB_RELEASES_URL: &str = "https://github.com/3xian/PinkDown/releases/download";

#[cfg(target_os = "windows")]
const RELEASE_ASSET: &str = "pinkdown-windows-x64-setup.exe";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RELEASE_ASSET: &str = "pinkdown-macos-arm64.dmg";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const RELEASE_ASSET: &str = "pinkdown-macos-x64.dmg";

/// Whether this build can download and apply a release package automatically.
pub const AUTO_INSTALL: bool = cfg!(any(target_os = "windows", target_os = "macos"));

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

/// A newer release the user has not yet accepted.
#[derive(Clone, Debug)]
pub struct AvailableUpdate {
    pub version: Version,
    /// GitHub tag for the release asset (e.g. `v1.4.0`).
    pub tag: String,
}

pub enum UpdateOutcome {
    UpToDate(Version),
    /// Call [`UpdateChecker::start_install`] after the user confirms when
    /// [`AUTO_INSTALL`] is true; otherwise open the releases page.
    Available(AvailableUpdate),
    /// Package downloaded, verified, and scheduled to apply after exit.
    InstallReady(Version),
}

/// Byte-level progress while a release package is downloading (or verifying).
#[derive(Clone, Copy, Debug)]
pub struct DownloadProgress {
    pub downloaded: u64,
    /// `Content-Length` when the server provides it.
    pub total: Option<u64>,
    pub phase: DownloadPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadPhase {
    Downloading,
    Verifying,
}

impl DownloadProgress {
    /// Status-bar line for an in-flight install of `version`.
    pub fn status_line(&self, version: &Version) -> String {
        match self.phase {
            DownloadPhase::Verifying => {
                format!("Verifying PinkDown v{version}\u{2026}")
            }
            DownloadPhase::Downloading => match self.total.filter(|&total| total > 0) {
                Some(total) => {
                    let pct = ((self.downloaded as f64 / total as f64) * 100.0)
                        .clamp(0.0, 100.0)
                        .round() as u32;
                    format!(
                        "Downloading PinkDown v{version}\u{2026} {} / {} ({pct}%)",
                        format_bytes(self.downloaded),
                        format_bytes(total),
                    )
                }
                None => format!(
                    "Downloading PinkDown v{version}\u{2026} {}",
                    format_bytes(self.downloaded),
                ),
            },
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let n = bytes as f64;
    if n >= MB {
        format!("{:.1} MB", n / MB)
    } else if n >= KB {
        format!("{:.0} KB", n / KB)
    } else {
        format!("{bytes} B")
    }
}

pub enum PollResult {
    Idle,
    Pending,
    /// Latest download progress since the previous poll (install path only).
    Progress(DownloadProgress),
    Ready(Result<UpdateOutcome, UpdateError>),
}

/// UI-facing update state machine (single source of truth for the toolbar/prompt).
#[derive(Default)]
pub enum UpdateUi {
    #[default]
    Idle,
    Checking,
    Available(AvailableUpdate),
    /// Install in flight; keeps the offer so a failed download can restore the prompt.
    Downloading(AvailableUpdate),
    Staged {
        version: Version,
    },
}

#[derive(Default)]
pub struct UpdateChecker {
    /// Finished outcome only — progress is a single shared slot so a stalled UI
    /// never queues unbounded download events.
    receiver: Option<Receiver<Result<UpdateOutcome, UpdateError>>>,
    progress: Option<Arc<Mutex<Option<DownloadProgress>>>>,
}

impl UpdateChecker {
    /// Starts a background version check against GitHub tags (no download).
    pub fn start(&mut self) -> bool {
        self.spawn(|_progress| check_for_update())
    }

    /// Downloads the release package, verifies its checksum, and schedules
    /// installation after PinkDown exits. Only meaningful when [`AUTO_INSTALL`].
    pub fn start_install(&mut self, available: AvailableUpdate) -> bool {
        if !AUTO_INSTALL {
            return false;
        }
        self.spawn(move |progress| download_and_stage_update(available, progress))
    }

    pub fn poll(&mut self) -> PollResult {
        let Some(receiver) = &self.receiver else {
            return PollResult::Idle;
        };

        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                self.progress = None;
                PollResult::Ready(result)
            }
            Err(TryRecvError::Empty) => {
                let latest = self
                    .progress
                    .as_ref()
                    .and_then(|slot| slot.lock().ok()?.take());
                match latest {
                    Some(progress) => PollResult::Progress(progress),
                    None => PollResult::Pending,
                }
            }
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                self.progress = None;
                PollResult::Ready(Err(UpdateError::new(
                    "Update check did not complete",
                )))
            }
        }
    }

    fn spawn(
        &mut self,
        work: impl FnOnce(ProgressCallback) -> Result<UpdateOutcome, UpdateError> + Send + 'static,
    ) -> bool {
        if self.receiver.is_some() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        let progress = Arc::new(Mutex::new(None));
        self.receiver = Some(receiver);
        self.progress = Some(Arc::clone(&progress));
        thread::spawn(move || {
            let on_progress: ProgressCallback = Box::new(move |value| {
                if let Ok(mut slot) = progress.lock() {
                    *slot = Some(value);
                }
            });
            let result = work(on_progress);
            let _ = sender.send(result);
        });
        true
    }
}

/// Callback used by the download worker to report progress to the UI thread.
type ProgressCallback = Box<dyn FnMut(DownloadProgress) + Send>;

#[derive(Deserialize)]
struct GitHubTag {
    name: String,
}

fn check_for_update() -> Result<UpdateOutcome, UpdateError> {
    let (latest_tag, latest_version) = latest_github_tag()?;
    let current_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| UpdateError::new(format!("Invalid current version: {error}")))?;

    if latest_version <= current_version {
        return Ok(UpdateOutcome::UpToDate(current_version));
    }

    Ok(UpdateOutcome::Available(AvailableUpdate {
        version: latest_version,
        tag: latest_tag,
    }))
}

fn download_and_stage_update(
    available: AvailableUpdate,
    mut on_progress: ProgressCallback,
) -> Result<UpdateOutcome, UpdateError> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let downloaded = download_release_asset(&available.tag, &mut on_progress)?;
        let schedule_result = {
            #[cfg(target_os = "windows")]
            {
                windows::schedule(&downloaded)
            }
            #[cfg(target_os = "macos")]
            {
                macos::schedule(&downloaded)
            }
        };
        if let Err(error) = schedule_result {
            let _ = std::fs::remove_file(downloaded);
            return Err(error);
        }
        Ok(UpdateOutcome::InstallReady(available.version))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = available;
        let _ = on_progress;
        Err(UpdateError::new(
            "Automatic install is not supported on this platform",
        ))
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

fn github_token_from(var: impl Fn(&str) -> Result<String, std::env::VarError>) -> Option<String> {
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

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn download_release_asset(
    tag: &str,
    on_progress: &mut ProgressCallback,
) -> Result<PathBuf, UpdateError> {
    use std::{
        fs,
        io::{Read, Write},
    };

    let asset_url = format!("{GITHUB_RELEASES_URL}/{tag}/{RELEASE_ASSET}");
    let checksum_url = format!("{asset_url}.sha256");
    let expected_checksum = download_text(&checksum_url)?
        .split_whitespace()
        .next()
        .filter(|checksum| checksum.len() == 64 && checksum.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| UpdateError::new("Release checksum is missing or invalid"))?
        .to_ascii_lowercase();
    let destination =
        std::env::temp_dir().join(format!("pinkdown-{tag}-{}-{RELEASE_ASSET}", std::process::id()));

    let result = (|| {
        const MAX_BYTES: u64 = 256 * 1024 * 1024;

        let response = ureq::get(&asset_url)
            .set("User-Agent", PINKDOWN_USER_AGENT)
            .call()
            .map_err(|error| UpdateError::new(format!("Could not download {tag}: {error}")))?;
        let total = response
            .header("Content-Length")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|&len| len > 0 && len <= MAX_BYTES);
        let mut source = response.into_reader();
        let mut file = fs::File::create(&destination)
            .map_err(|error| UpdateError::new(format!("Could not create update file: {error}")))?;

        let mut buffer = [0u8; 64 * 1024];
        let mut downloaded = 0u64;
        let mut last_report = Instant::now();

        // Immediate 0% so the status bar is not stuck on a bare "Downloading…".
        on_progress(DownloadProgress {
            downloaded: 0,
            total,
            phase: DownloadPhase::Downloading,
        });

        loop {
            let bytes_read = source
                .read(&mut buffer)
                .map_err(|error| UpdateError::new(format!("Could not save update: {error}")))?;
            if bytes_read == 0 {
                break;
            }
            let next = downloaded.saturating_add(bytes_read as u64);
            if next > MAX_BYTES {
                return Err(UpdateError::new(
                    "Downloaded update exceeds the 256 MiB limit",
                ));
            }
            file.write_all(&buffer[..bytes_read])
                .map_err(|error| UpdateError::new(format!("Could not save update: {error}")))?;
            downloaded = next;

            // Throttle UI events (~10/s); a final snapshot is sent after the loop.
            if last_report.elapsed() >= Duration::from_millis(100) {
                on_progress(DownloadProgress {
                    downloaded,
                    total,
                    phase: DownloadPhase::Downloading,
                });
                last_report = Instant::now();
            }
        }

        on_progress(DownloadProgress {
            downloaded,
            total: total.or(Some(downloaded)),
            phase: DownloadPhase::Downloading,
        });
        on_progress(DownloadProgress {
            downloaded,
            total: total.or(Some(downloaded)),
            phase: DownloadPhase::Verifying,
        });

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

#[cfg(any(target_os = "windows", target_os = "macos"))]
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

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn sha256_file(path: &Path) -> Result<String, UpdateError> {
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

/// Shared handoff: wait until the helper creates `ready`, or fail if it exits /
/// times out before acknowledging.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn wait_for_updater_ready(
    child: &mut Child,
    ready: &Path,
    script: &Path,
    log: &Path,
) -> Result<(), UpdateError> {
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
    let _ = std::fs::remove_file(script);
    Err(UpdateError::new("Updater did not acknowledge the handoff"))
}

/// Paths used by both platform helpers for the ready/log/script handshake.
#[cfg(any(target_os = "windows", target_os = "macos"))]
struct UpdaterPaths {
    script: PathBuf,
    ready: PathBuf,
    log: PathBuf,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn updater_paths(extension: &str) -> UpdaterPaths {
    let temp = std::env::temp_dir();
    let process_id = std::process::id();
    UpdaterPaths {
        script: temp.join(format!("pinkdown-update-{process_id}.{extension}")),
        ready: temp.join(format!("pinkdown-update-{process_id}.ready")),
        log: temp.join(format!("pinkdown-update-{process_id}.log")),
    }
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
        assert_eq!(
            authorization.as_deref(),
            Some("Authorization: Bearer secret-token")
        );
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
        let installer = include_str!("../../installer/pinkdown.iss");

        assert!(installer.contains("Software\\Classes\\.md\\OpenWithProgids"));
        assert!(installer.contains("Software\\RegisteredApplications"));
        assert!(installer.contains("registeredAppUser=PinkDown"));
        assert!(installer.contains("Tasks: associate_md"));
        assert!(installer.contains("{#MyAppExeName}\"\" \"\"%1"));
    }

    #[test]
    fn mac_release_packages_a_dmg_with_a_retina_icon() {
        let plist = include_str!("../../installer/macos/Info.plist");
        let packager = include_str!("../../installer/macos/package.ps1");
        let workflow = include_str!("../../.github/workflows/release.yml");

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

    #[test]
    fn available_update_always_carries_a_tag() {
        let available = AvailableUpdate {
            version: Version::parse("1.2.3").unwrap(),
            tag: "v1.2.3".into(),
        };
        assert_eq!(available.tag, "v1.2.3");
    }

    #[test]
    fn download_progress_status_includes_percent_when_total_is_known() {
        let version = Version::parse("1.6.0").unwrap();
        let line = DownloadProgress {
            downloaded: 5 * 1024 * 1024,
            total: Some(20 * 1024 * 1024),
            phase: DownloadPhase::Downloading,
        }
        .status_line(&version);
        assert!(line.contains("Downloading PinkDown v1.6.0"));
        assert!(line.contains("25%"), "line was: {line}");
        assert!(line.contains("5.0 MB") && line.contains("20.0 MB"), "line was: {line}");
    }

    #[test]
    fn download_progress_status_without_total_shows_bytes_only() {
        let version = Version::parse("1.6.0").unwrap();
        let line = DownloadProgress {
            downloaded: 1500,
            total: None,
            phase: DownloadPhase::Downloading,
        }
        .status_line(&version);
        assert!(line.contains("1 KB") || line.contains("1500 B"), "line was: {line}");
        assert!(!line.contains('%'), "line was: {line}");
    }

    #[test]
    fn download_progress_verifying_phase_is_distinct() {
        let version = Version::parse("1.6.0").unwrap();
        let line = DownloadProgress {
            downloaded: 10,
            total: Some(10),
            phase: DownloadPhase::Verifying,
        }
        .status_line(&version);
        assert!(line.contains("Verifying"), "line was: {line}");
    }
}
