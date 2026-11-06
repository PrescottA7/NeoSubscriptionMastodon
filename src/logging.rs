// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use std::io::IsTerminal;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Rotate the log file once it grows past this size.
const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

/// Installs the global logger.
///
/// - respects `RUST_LOG`, defaulting to `info`;
/// - writes to stdout when it is a terminal, so interactive runs look normal;
/// - otherwise writes to `<data_root>/logs/nsm.log`, because a double-clicked
///   binary has no visible stdout and its output would be lost.
pub fn init(data_root: &std::path::Path) -> anyhow::Result<Option<std::path::PathBuf>> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // `sqlx` logs every statement at INFO; keep the default readable but
        // still overridable with RUST_LOG.
        EnvFilter::new("info,sqlx=warn,sea_orm=warn")
    });

    if std::io::stdout().is_terminal() {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .init();
        return Ok(None);
    }

    let log_dir = data_root.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let path = log_dir.join("nsm.log");
    rotate_if_needed(&path, &log_dir)?;

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .with_target(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();

    Ok(Some(path))
}

fn rotate_if_needed(path: &std::path::Path, dir: &std::path::Path) -> std::io::Result<()> {
    let too_big = std::fs::metadata(path)
        .map(|meta| meta.len() > MAX_LOG_BYTES)
        .unwrap_or(false);
    if too_big {
        let _ = std::fs::rename(path, dir.join("nsm.log.1"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rotation_moves_an_oversized_log() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("nsm.log");
        std::fs::write(&path, vec![b'x'; (MAX_LOG_BYTES + 1) as usize]).expect("write");

        rotate_if_needed(&path, temp.path()).expect("rotate");

        assert!(!path.exists());
        assert!(temp.path().join("nsm.log.1").exists());
    }

    #[test]
    fn small_logs_are_left_alone() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("nsm.log");
        std::fs::write(&path, b"hello").expect("write");

        rotate_if_needed(&path, temp.path()).expect("rotate");

        assert!(path.exists());
        assert!(!temp.path().join("nsm.log.1").exists());
    }
}
