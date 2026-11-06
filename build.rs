// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use std::path::Path;
use std::process::{Command, Output};

fn emit_cargo_warnings(output: &[u8]) {
    for line in String::from_utf8_lossy(output).lines() {
        if !line.trim().is_empty() {
            println!("cargo:warning={}", line);
        }
    }
}

fn run_pnpm(frontend_dir: &Path, args: &[&str]) -> std::io::Result<Output> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("pnpm");
        command
    } else {
        Command::new("pnpm")
    };

    command.args(args).current_dir(frontend_dir).output()
}

fn has_pnpm() -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "pnpm", "--version"])
            .output()
    } else {
        Command::new("pnpm").arg("--version").output()
    };

    output.map(|output| output.status.success()).unwrap_or(false)
}

fn has_bundle() -> bool {
    Path::new("frontend/dist/index.html").exists()
}

fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/public");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=build.rs");

    let frontend_dir = Path::new("frontend");
    if !frontend_dir.exists() {
        println!("cargo:warning=frontend directory not found, skipping UI build");
        return;
    }

    if !has_pnpm() {
        if has_bundle() {
            println!("cargo:warning=pnpm not found, reusing the existing frontend/dist");
        } else {
            println!("cargo:warning=pnpm not found and frontend/dist is missing, the admin UI will be unavailable");
        }
        return;
    }

    // Dependency installation needs network access, so it stays opt-in.
    let install = std::env::var("NSM_FRONTEND_INSTALL")
        .map(|value| value != "0")
        .unwrap_or(false);

    let changed = frontend_sources_changed();

    if !changed && has_bundle() {
        println!("cargo:warning=frontend/dist is up to date, skipping the UI build");
        return;
    }

    if install {
        println!("cargo:warning=installing frontend dependencies...");
        match run_pnpm(frontend_dir, &["install"]) {
            Ok(output) if output.status.success() => {
                emit_cargo_warnings(&output.stdout);
                emit_cargo_warnings(&output.stderr);
            }
            Ok(output) => {
                emit_cargo_warnings(&output.stdout);
                emit_cargo_warnings(&output.stderr);
                println!(
                    "cargo:warning=frontend dependency installation failed: {}",
                    output.status
                );
                return;
            }
            Err(error) => {
                println!("cargo:warning=failed to run pnpm install: {error}");
                return;
            }
        }
    }

    println!("cargo:warning=building frontend...");
    match run_pnpm(frontend_dir, &["run", "build"]) {
        Ok(output) if output.status.success() => {
            emit_cargo_warnings(&output.stdout);
            emit_cargo_warnings(&output.stderr);
            println!("cargo:warning=frontend build completed");
        }
        Ok(output) => {
            emit_cargo_warnings(&output.stdout);
            emit_cargo_warnings(&output.stderr);
            if has_bundle() && !changed {
                println!("cargo:warning=frontend build failed, reusing the existing frontend/dist");
            } else {
                panic!("frontend build failed with exit code {}", output.status);
            }
        }
        Err(error) => panic!("failed to run frontend build: {error}"),
    }
}

/// True when a tracked frontend source is newer than the built bundle.
fn frontend_sources_changed() -> bool {
    let bundle = Path::new("frontend/dist/index.html");
    let Ok(bundle_time) = bundle.metadata().and_then(|meta| meta.modified()) else {
        return true;
    };

    fn newest(dir: &Path) -> Option<std::time::SystemTime> {
        let mut result: Option<std::time::SystemTime> = None;
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            let time = if meta.is_dir() {
                newest(&path)
            } else {
                meta.modified().ok()
            };
            if let Some(time) = time {
                result = Some(match result {
                    Some(current) if current > time => current,
                    _ => time,
                });
            }
        }
        result
    }

    ["frontend/src", "frontend/public", "frontend/index.html"]
        .iter()
        .filter_map(|path| {
            let path = Path::new(path);
            if path.is_dir() {
                newest(path)
            } else {
                path.metadata().and_then(|meta| meta.modified()).ok()
            }
        })
        .any(|time| time > bundle_time)
}
