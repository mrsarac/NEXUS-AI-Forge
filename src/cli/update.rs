//! Self-update command for NEXUS AI Forge
//!
//! Checks GitHub releases for newer versions and updates the binary.

#![allow(dead_code)]

use anyhow::{Context, Result, anyhow};
use std::io::{self, Write};
use std::fs;
use std::env;

// ANSI color codes
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const PRIMARY: &str = "\x1b[38;2;100;181;246m";      // #64B5F6
    pub const SUCCESS: &str = "\x1b[38;2;165;214;167m";      // #A5D6A7
    pub const ERROR: &str = "\x1b[38;2;239;154;154m";        // #EF9A9A
    pub const WARNING: &str = "\x1b[38;2;255;202;40m";       // #FFCA28
    pub const MUTED: &str = "\x1b[38;2;84;110;122m";         // #546E7A
    pub const FG: &str = "\x1b[38;2;212;212;215m";           // #D4D4D7
}

mod symbols {
    pub const UPDATE: &str = "󰚰";
    pub const SUCCESS: &str = "󰄂";
    pub const ERROR: &str = "󰅚";
    pub const INFO: &str = "󰋼";
    pub const DOWNLOAD: &str = "󰇚";
    pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

const GITHUB_REPO: &str = "mrsarac/NEXUS-AI-Forge";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub Release API response
#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
    body: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// Run the update command
pub async fn run(check_only: bool, force: bool) -> Result<()> {
    print_header();

    // Check for latest version
    print_status("Checking for updates...");
    let latest = fetch_latest_release().await?;
    clear_line();

    let latest_version = latest.tag_name.trim_start_matches('v');
    let current_version = CURRENT_VERSION;

    // Compare versions
    let update_available = is_newer_version(latest_version, current_version);

    if !update_available && !force {
        print_up_to_date(current_version);
        return Ok(());
    }

    if update_available {
        print_update_available(current_version, latest_version, &latest);
    } else if force {
        println!(
            "\n{}  {} Forcing reinstall of v{}{}",
            colors::WARNING, symbols::INFO, current_version, colors::RESET
        );
    }

    if check_only {
        return Ok(());
    }

    // Find the right asset for this platform
    let asset = find_platform_asset(&latest.assets)?;

    // Confirm update
    if !force {
        print!("\n{}  Do you want to update? [y/N]: {}", colors::FG, colors::RESET);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("\n{}  {} Update cancelled{}", colors::MUTED, symbols::INFO, colors::RESET);
            return Ok(());
        }
    }

    // Download and install
    println!();
    print_downloading(&asset.name, asset.size);

    let binary_data = download_binary(&asset.browser_download_url).await?;
    clear_line();

    print_installing();
    install_binary(&binary_data)?;
    clear_line();

    print_success(latest_version);

    Ok(())
}

/// Check if only checking for updates (no install)
pub async fn check() -> Result<()> {
    run(true, false).await
}

/// Get GitHub token from environment or gh CLI
fn get_github_token() -> Option<String> {
    // First try environment variable
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        return Some(token);
    }
    if let Ok(token) = env::var("GH_TOKEN") {
        return Some(token);
    }

    // Try to get token from gh CLI
    if let Ok(output) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }

    None
}

/// Fetch the latest release from GitHub
async fn fetch_latest_release() -> Result<GitHubRelease> {
    let client = reqwest::Client::builder()
        .user_agent("nexus-forge-updater")
        .build()?;

    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);

    let mut request = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json");

    // Add auth token for private repos
    if let Some(token) = get_github_token() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .context("Failed to connect to GitHub")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status.as_u16() == 404 {
            return Err(anyhow!(
                "Release not found. If this is a private repo, ensure you have gh CLI authenticated or set GITHUB_TOKEN."
            ));
        }

        return Err(anyhow!("GitHub API error: {} - {}", status, body));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse GitHub release")?;

    Ok(release)
}

/// Compare semantic versions (returns true if latest > current)
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for i in 0..3 {
        let l = latest_parts.get(i).unwrap_or(&0);
        let c = current_parts.get(i).unwrap_or(&0);

        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }

    false
}

/// Find the right binary asset for this platform
fn find_platform_asset(assets: &[GitHubAsset]) -> Result<&GitHubAsset> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    // Map to expected asset names
    let expected_name = match (os, arch) {
        ("macos", "aarch64") => "nexus-darwin-arm64",
        ("macos", "x86_64") => "nexus-darwin-x64",
        ("linux", "x86_64") => "nexus-linux-x64",
        ("linux", "aarch64") => "nexus-linux-arm64",
        ("windows", "x86_64") => "nexus-windows-x64.exe",
        _ => return Err(anyhow!("Unsupported platform: {}-{}", os, arch)),
    };

    assets
        .iter()
        .find(|a| a.name == expected_name || a.name.contains(expected_name))
        .ok_or_else(|| anyhow!(
            "No binary found for {}-{}. Available: {:?}",
            os, arch,
            assets.iter().map(|a| &a.name).collect::<Vec<_>>()
        ))
}

/// Download the binary from GitHub (supports private repos)
async fn download_binary(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent("nexus-forge-updater")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let mut request = client
        .get(url)
        .header("Accept", "application/octet-stream");

    // Add auth token for private repos
    if let Some(token) = get_github_token() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .context("Failed to download binary")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed: {}. For private repos, ensure gh CLI is authenticated.",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read binary data")?;

    Ok(bytes.to_vec())
}

/// Install the new binary
fn install_binary(data: &[u8]) -> Result<()> {
    // Get current binary path
    let current_exe = env::current_exe()
        .context("Failed to get current executable path")?;

    // Create backup
    let backup_path = current_exe.with_extension("old");
    if backup_path.exists() {
        fs::remove_file(&backup_path).ok();
    }

    // Try to rename current binary to backup
    fs::rename(&current_exe, &backup_path)
        .context("Failed to backup current binary. Try running with sudo.")?;

    // Write new binary
    match fs::write(&current_exe, data) {
        Ok(_) => {
            // Set executable permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&current_exe)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&current_exe, perms)?;
            }

            // Remove backup on success
            fs::remove_file(&backup_path).ok();
            Ok(())
        }
        Err(e) => {
            // Restore backup on failure
            fs::rename(&backup_path, &current_exe).ok();
            Err(anyhow!("Failed to write new binary: {}. Try running with sudo.", e))
        }
    }
}

// ============================================
// UI Functions
// ============================================

fn print_header() {
    println!();
    println!(
        "{}{}  {} NEXUS AI Forge Updater{}",
        colors::PRIMARY, colors::BOLD, symbols::UPDATE, colors::RESET
    );
    println!(
        "{}  ╰{}─{}",
        colors::MUTED, "─".repeat(40), colors::RESET
    );
    println!();
}

fn print_status(message: &str) {
    print!(
        "\r{}  {} {}{}",
        colors::MUTED, symbols::SPINNER[0], message, colors::RESET
    );
    io::stdout().flush().ok();
}

fn clear_line() {
    print!("\r{}\r", " ".repeat(60));
    io::stdout().flush().ok();
}

fn print_up_to_date(version: &str) {
    println!(
        "{}  {} You're up to date!{}",
        colors::SUCCESS, symbols::SUCCESS, colors::RESET
    );
    println!(
        "{}  Current version: v{}{}",
        colors::MUTED, version, colors::RESET
    );
    println!();
}

fn print_update_available(current: &str, latest: &str, release: &GitHubRelease) {
    println!(
        "{}{}  {} Update available!{}",
        colors::WARNING, colors::BOLD, symbols::UPDATE, colors::RESET
    );
    println!();
    println!(
        "{}  Current: {}v{}{}",
        colors::MUTED, colors::FG, current, colors::RESET
    );
    println!(
        "{}  Latest:  {}{}v{}{}",
        colors::MUTED, colors::SUCCESS, colors::BOLD, latest, colors::RESET
    );
    println!();

    // Show release notes if available
    if let Some(body) = &release.body {
        let lines: Vec<&str> = body.lines().take(5).collect();
        if !lines.is_empty() {
            println!(
                "{}  Release notes:{}",
                colors::MUTED, colors::RESET
            );
            for line in lines {
                println!("{}  │ {}{}", colors::MUTED, colors::FG, line);
            }
            if body.lines().count() > 5 {
                println!("{}  │ ...{}", colors::MUTED, colors::RESET);
            }
            println!();
        }
    }

    println!(
        "{}  Details: {}{}",
        colors::MUTED, release.html_url, colors::RESET
    );
}

fn print_downloading(name: &str, size: u64) {
    let size_mb = size as f64 / 1024.0 / 1024.0;
    print!(
        "\r{}  {} Downloading {} ({:.1} MB)...{}",
        colors::PRIMARY, symbols::DOWNLOAD, name, size_mb, colors::RESET
    );
    io::stdout().flush().ok();
}

fn print_installing() {
    print!(
        "\r{}  {} Installing...{}",
        colors::PRIMARY, symbols::SPINNER[0], colors::RESET
    );
    io::stdout().flush().ok();
}

fn print_success(version: &str) {
    println!(
        "{}{}  {} Successfully updated to v{}!{}",
        colors::SUCCESS, colors::BOLD, symbols::SUCCESS, version, colors::RESET
    );
    println!();
    println!(
        "{}  Run 'nexus --version' to verify.{}",
        colors::MUTED, colors::RESET
    );
    println!();
}
