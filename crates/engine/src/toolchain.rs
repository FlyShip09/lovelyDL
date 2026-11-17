//! Finds, installs, verifies and health-checks yt-dlp, ffmpeg/ffprobe and Deno.
//!
//! Managed tools live in `<bin>/<tool>/<version>/`. A `current` file names the version in
//! use and `previous` the one before it, which is kept for rollback. Installs are atomic:
//! download, verify the SHA-256, unpack into a staging folder, health-check, then switch.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::model::{ReleaseChannel, Settings, ToolId, ToolOrigin};
use crate::ytdlp::run;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("could not reach the download server: {0}")]
    Network(String),
    #[error("GitHub is limiting requests from this network right now; try again in an hour")]
    RateLimited,
    #[error("the latest release has no download for this system")]
    NoAsset,
    #[error("the release publishes no checksum, so the download cannot be verified")]
    NoChecksum,
    #[error("the download is corrupt or was tampered with (expected {expected}, got {actual})")]
    Checksum { expected: String, actual: String },
    #[error("could not write to the tools folder: {0}")]
    Io(String),
    #[error("could not unpack the download: {0}")]
    Archive(String),
    #[error("the installed tool does not run: {0}")]
    Unhealthy(String),
    #[error("there is no previous version to go back to")]
    NoPrevious,
}

impl From<std::io::Error> for ToolError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<reqwest::Error> for ToolError {
    fn from(e: reqwest::Error) -> Self {
        match e.status().map(|s| s.as_u16()) {
            Some(403 | 429) => Self::RateLimited,
            _ => Self::Network(e.without_url().to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

pub struct Toolchain {
    pub bin: PathBuf,
    /// GitHub API base URL. Tests point it at a local server.
    pub api: String,
    pub http: reqwest::Client,
}

fn folder(id: ToolId) -> &'static str {
    match id {
        ToolId::Ytdlp => "yt-dlp",
        ToolId::Ffmpeg => "ffmpeg",
        ToolId::Deno => "deno",
    }
}

/// File name of a tool's executable on this system.
pub fn exe(id: ToolId) -> String {
    format!("{}{}", folder(id), std::env::consts::EXE_SUFFIX)
}

/// Version strings come from the network and become folder names.
fn safe(version: &str) -> String {
    let cleaned: String = version
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if cleaned.trim_matches('.').is_empty() { "unknown".into() } else { cleaned }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Finds `<hash>  <name>` in a `SHA2-256SUMS` file.
fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && hash.len() == 64).then(|| hash.to_string())
    })
}

pub fn parse_version(id: ToolId, stdout: &str) -> Option<String> {
    let line = stdout.lines().next()?.trim();
    let version = match id {
        ToolId::Ytdlp => line,
        ToolId::Ffmpeg => {
            let token = line.strip_prefix("ffmpeg version ")?.split_whitespace().next()?;
            // "9.0.2-essentials_build-www.gyan.dev" is 9.0.2; git builds ("N-1234-g…") stay whole.
            if token.starts_with(|c: char| c.is_ascii_digit()) { token.split('-').next()? } else { token }
        }
        // "deno 2.9.8 (stable, …)"; other runtimes print just a version, such as "v22.3.0".
        ToolId::Deno => line.strip_prefix("deno ").unwrap_or(line).split_whitespace().next()?,
    };
    (!version.is_empty() && version.len() < 80).then(|| version.to_string())
}

/// Runs the tool's version command and returns the version, or why it does not work.
pub async fn health(id: ToolId, path: &Path) -> Result<String, String> {
    let flag = if id == ToolId::Ffmpeg { "-version" } else { "--version" };
    let out = run::output(path, &[flag.to_string()], 30).await.map_err(|e| e.to_string())?;
    if out.code != Some(0) {
        let reason = out.stderr.lines().next().unwrap_or("it exited with an error").trim().to_string();
        return Err(reason);
    }
    parse_version(id, &out.stdout).ok_or_else(|| "it printed no version".to_string())
}

/// Looks for `name` in the folders on PATH. Blocking.
pub fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|dir| dir.join(name)).find(|p| p.is_file())
}

/// The runtime name and binary a `name:path` setting points to.
pub fn custom_runtime(setting: &str) -> Option<(String, PathBuf)> {
    let (name, path) = setting.split_once(':')?;
    let known = ["deno", "node", "quickjs", "bun"].contains(&name);
    (known && !path.is_empty()).then(|| (name.to_string(), PathBuf::from(path)))
}

impl Toolchain {
    pub fn new(bin: PathBuf, user_agent: &str) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .connect_timeout(std::time::Duration::from_secs(20))
            .read_timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self { bin, api: "https://api.github.com".into(), http }
    }

    fn dir(&self, id: ToolId) -> PathBuf {
        self.bin.join(folder(id))
    }

    /// The managed copy in use, if one is installed. Blocking.
    pub fn managed(&self, id: ToolId) -> Option<PathBuf> {
        let dir = self.dir(id);
        let version = std::fs::read_to_string(dir.join("current")).ok()?;
        let path = dir.join(safe(version.trim())).join(exe(id));
        path.is_file().then_some(path)
    }

    /// Where a tool should be run from, by the user's settings. Blocking.
    ///
    /// A path the user set always wins. ffmpeg already on PATH is used rather than
    /// downloading 100 MB; yt-dlp and Deno prefer the managed copy, which the app keeps
    /// current, unless the user asks for the system ones.
    pub fn locate(&self, id: ToolId, settings: &Settings) -> Option<(PathBuf, ToolOrigin)> {
        let custom = match id {
            ToolId::Ytdlp => (!settings.custom_ytdlp.is_empty()).then(|| PathBuf::from(&settings.custom_ytdlp)),
            ToolId::Ffmpeg => {
                (!settings.custom_ffmpeg_dir.is_empty()).then(|| Path::new(&settings.custom_ffmpeg_dir).join(exe(id)))
            }
            ToolId::Deno => custom_runtime(&settings.custom_js_runtime).map(|(_, path)| path),
        };
        if let Some(path) = custom {
            return path.is_file().then_some((path, ToolOrigin::Custom));
        }
        let system = || on_path(&exe(id)).map(|p| (p, ToolOrigin::System));
        let managed = || self.managed(id).map(|p| (p, ToolOrigin::Managed));
        if settings.use_system_tools {
            system().or_else(managed)
        } else if id == ToolId::Ffmpeg {
            managed().or_else(system)
        } else {
            managed()
        }
    }

    async fn get(&self, url: &str) -> Result<reqwest::Response, ToolError> {
        Ok(self.http.get(url).header("Accept", "application/vnd.github+json").send().await?.error_for_status()?)
    }

    /// The newest release of a tool and the checksum its download must match.
    pub async fn latest(&self, id: ToolId, channel: ReleaseChannel) -> Result<Release, ToolError> {
        let repo = match (id, channel) {
            (ToolId::Ytdlp, ReleaseChannel::Stable) => "yt-dlp/yt-dlp",
            (ToolId::Ytdlp, ReleaseChannel::Nightly) => "yt-dlp/yt-dlp-nightly-builds",
            (ToolId::Deno, _) => "denoland/deno",
            (ToolId::Ffmpeg, _) => "GyanD/codexffmpeg",
        };
        let release: GhRelease = self.get(&format!("{}/repos/{repo}/releases/latest", self.api)).await?.json().await?;
        let asset = release
            .assets
            .iter()
            .find(|a| match id {
                ToolId::Ytdlp => a.name == "yt-dlp_win.zip",
                ToolId::Deno => a.name == "deno-x86_64-pc-windows-msvc.zip",
                ToolId::Ffmpeg => a.name.ends_with("-essentials_build.zip"),
            })
            .ok_or(ToolError::NoAsset)?;
        // The publisher's own checksum file when there is one, else GitHub's asset digest.
        let sha256 = match release.assets.iter().find(|a| a.name == "SHA2-256SUMS") {
            Some(sums) => {
                let text = self.get(&sums.browser_download_url).await?.text().await?;
                sum_for(&text, &asset.name)
            }
            None => asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")).map(String::from),
        }
        .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or(ToolError::NoChecksum)?;
        Ok(Release {
            version: release.tag_name.trim_start_matches('v').to_string(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
            sha256,
        })
    }

    /// Downloads to `dest` and returns the SHA-256 of what arrived.
    async fn download(
        &self,
        url: &str,
        dest: &Path,
        size: u64,
        progress: &(dyn Fn(f64) + Send + Sync),
    ) -> Result<String, ToolError> {
        let mut response = self.http.get(url).send().await?.error_for_status()?;
        let total = response.content_length().unwrap_or(size).max(1) as f64;
        let mut file = tokio::fs::File::create(dest).await?;
        let mut hasher = Sha256::new();
        let mut done = 0.0;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            done += chunk.len() as f64;
            progress((done / total).min(1.0));
        }
        file.flush().await?;
        Ok(hex(&hasher.finalize()))
    }

    /// Installs a release and makes it the version in use. Returns the executable's path.
    pub async fn install(
        &self,
        id: ToolId,
        release: &Release,
        progress: impl Fn(f64) + Send + Sync,
    ) -> Result<PathBuf, ToolError> {
        let dir = self.dir(id);
        let version = safe(&release.version);
        tokio::fs::create_dir_all(&dir).await?;
        let archive = dir.join(format!(".download-{version}.zip"));
        let staging = dir.join(format!(".staging-{version}"));
        let target = dir.join(&version);

        let downloaded = self.download(&release.url, &archive, release.size, &progress).await;
        let actual = match downloaded {
            Ok(actual) if actual.eq_ignore_ascii_case(&release.sha256) => actual,
            other => {
                let _ = tokio::fs::remove_file(&archive).await;
                return Err(match other {
                    Ok(actual) => ToolError::Checksum { expected: release.sha256.clone(), actual },
                    Err(error) => error,
                });
            }
        };
        tracing::info!(tool = folder(id), version, sha256 = actual, "download verified");

        let (a, s, t) = (archive.clone(), staging.clone(), target.clone());
        let unpacked = tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
            let _ = std::fs::remove_dir_all(&s);
            unpack(id, &a, &s)?;
            if !s.join(exe(id)).is_file() {
                return Err(ToolError::Archive(format!("it does not contain {}", exe(id))));
            }
            let _ = std::fs::remove_dir_all(&t);
            std::fs::rename(&s, &t)?;
            Ok(())
        })
        .await
        .map_err(|e| ToolError::Io(e.to_string()))
        .and_then(|r| r);
        let _ = tokio::fs::remove_file(&archive).await;
        if let Err(error) = unpacked {
            let _ = tokio::fs::remove_dir_all(&staging).await;
            return Err(error);
        }

        // Only a version that actually runs becomes the one in use.
        let binary = target.join(exe(id));
        if let Err(reason) = health(id, &binary).await {
            let _ = tokio::fs::remove_dir_all(&target).await;
            return Err(ToolError::Unhealthy(reason));
        }
        tokio::task::spawn_blocking(move || switch(&dir, &version))
            .await
            .map_err(|e| ToolError::Io(e.to_string()))??;
        Ok(binary)
    }

    /// Goes back to the version that was in use before the current one.
    pub async fn rollback(&self, id: ToolId) -> Result<PathBuf, ToolError> {
        let dir = self.dir(id);
        let binary = tokio::task::spawn_blocking(move || -> Result<PathBuf, ToolError> {
            let previous = std::fs::read_to_string(dir.join("previous")).map_err(|_| ToolError::NoPrevious)?;
            let previous = safe(previous.trim());
            let binary = dir.join(&previous).join(exe(id));
            if !binary.is_file() {
                return Err(ToolError::NoPrevious);
            }
            switch(&dir, &previous)?;
            Ok(binary)
        })
        .await
        .map_err(|e| ToolError::Io(e.to_string()))??;
        Ok(binary)
    }
}

/// Points `current` at `version`, remembers the old one as `previous`, and deletes every
/// other version folder.
fn switch(dir: &Path, version: &str) -> Result<(), ToolError> {
    let old = std::fs::read_to_string(dir.join("current")).ok().map(|v| safe(v.trim()));
    if let Some(old) = old.as_deref().filter(|old| *old != version) {
        std::fs::write(dir.join("previous"), old)?;
    }
    let tmp = dir.join("current.tmp");
    std::fs::write(&tmp, version)?;
    std::fs::rename(&tmp, dir.join("current"))?;
    let previous = std::fs::read_to_string(dir.join("previous")).ok().map(|v| safe(v.trim()));
    for entry in std::fs::read_dir(dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let keep = name == version || Some(name.as_str()) == previous.as_deref();
        if entry.path().is_dir() && !keep {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}

/// Unpacks what a tool needs from its zip. Entries that would land outside `dest` are skipped.
fn unpack(id: ToolId, archive: &Path, dest: &Path) -> Result<(), ToolError> {
    let bad = |e: zip::result::ZipError| ToolError::Archive(e.to_string());
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(bad)?;
    std::fs::create_dir_all(dest)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(bad)?;
        let Some(inner) = entry.enclosed_name() else { continue };
        if entry.is_dir() {
            continue;
        }
        let relative = match id {
            // The ffmpeg zip has docs, presets and ffplay; only two binaries are needed.
            ToolId::Ffmpeg => {
                let name = inner.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if name != exe(ToolId::Ffmpeg) && name != format!("ffprobe{}", std::env::consts::EXE_SUFFIX) {
                    continue;
                }
                PathBuf::from(name)
            }
            _ => inner,
        };
        let out = dest.join(relative);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(&out)?;
        std::io::copy(&mut entry.by_ref(), &mut file)?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_from_real_tool_output() {
        assert_eq!(parse_version(ToolId::Ytdlp, "2026.10.26\n").as_deref(), Some("2026.10.26"));
        assert_eq!(
            parse_version(ToolId::Ffmpeg, "ffmpeg version 9.0.2-full_build-www.gyan.dev Copyright (c) 2000-2026 the FFmpeg developers\nbuilt with gcc").as_deref(),
            Some("9.0.2")
        );
        assert_eq!(
            parse_version(ToolId::Ffmpeg, "ffmpeg version N-118193-g5f38c82536 Copyright").as_deref(),
            Some("N-118193-g5f38c82536")
        );
        assert_eq!(
            parse_version(ToolId::Deno, "deno 2.9.8 (stable, release, x86_64-pc-windows-msvc)\nv8 14.1").as_deref(),
            Some("2.9.8")
        );
        assert_eq!(parse_version(ToolId::Deno, "v22.3.0").as_deref(), Some("v22.3.0"));
        assert_eq!(parse_version(ToolId::Ffmpeg, "not ffmpeg"), None);
        assert_eq!(parse_version(ToolId::Ytdlp, ""), None);
    }

    #[test]
    fn checksum_file_lookup() {
        let sums = "e51e69e89489f199530f13d32a4342da3ddb2cf34842f364603a066db05afbf3  yt-dlp\n\
                    fd51b212cf8380c7a52ee08c0d6be7f3c59b2f2c832eaf7b45acfae6eab57419  yt-dlp_win.zip\n\
                    5eaa4c61f3c09189f3a551bf1855c13aad61fb865cc33f06012a6c2dbe287a45 *yt-dlp_win_arm64.zip\n";
        assert_eq!(
            sum_for(sums, "yt-dlp_win.zip").as_deref(),
            Some("fd51b212cf8380c7a52ee08c0d6be7f3c59b2f2c832eaf7b45acfae6eab57419")
        );
        assert!(sum_for(sums, "yt-dlp_win_arm64.zip").is_some());
        assert_eq!(sum_for(sums, "yt-dlp_win"), None);
        assert_eq!(sum_for("short  yt-dlp_win.zip", "yt-dlp_win.zip"), None);
    }

    #[test]
    fn versions_cannot_escape_the_tools_folder() {
        assert_eq!(safe("2026.10.26"), "2026.10.26");
        assert_eq!(safe("../../evil"), ".._.._evil");
        assert_eq!(safe(".."), "unknown");
        assert_eq!(safe(""), "unknown");
        assert_eq!(safe(r"a\b:c"), "a_b_c");
    }

    #[test]
    fn custom_runtimes() {
        assert_eq!(custom_runtime(r"node:C:\node\node.exe"), Some(("node".into(), PathBuf::from(r"C:\node\node.exe"))));
        assert_eq!(custom_runtime(r"C:\node\node.exe"), None, "a drive letter is not a runtime name");
        assert_eq!(custom_runtime("perl:/usr/bin/perl"), None);
        assert_eq!(custom_runtime(""), None);
    }
}
