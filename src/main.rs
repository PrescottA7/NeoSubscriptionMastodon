// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use anyhow::Context;
use nsm::AppConfig;
use tracing::{info, warn};

fn main() {
    if let Err(error) = run() {
        // Logging may not be initialised yet, so always report fatally to stderr.
        eprintln!("fatal: {error:#}");
        tracing::error!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let config = AppConfig::from_cli().context("load configuration")?;

    // Directories must exist before the logger can write into `data_root`.
    config
        .ensure_directories()
        .context("create data directory")?;

    let log_file = nsm::logging::init(&config.data_root).context("initialise logging")?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        bind_addr = %config.bind_addr,
        data_root = %config.data_root.display(),
        "nsm starting"
    );

    match log_file.as_ref() {
        Some(path) => info!(path = %path.display(), "writing logs to file"),
        None => info!("logging to stdout"),
    }

    if config.created {
        info!("config.yml was missing, a commented template has been generated");
    }
    if let Some(warning) = config.insecure_transport_warning() {
        warn!("{warning}");
    }

    let bind_addr = config.bind_addr.clone();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;

    runtime.block_on(async move {
        let app = nsm::build_app(config).await.context("build application")?;
        let listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .with_context(|| {
                format!(
                    "cannot bind {bind_addr}. Another process may already use this port, \
                     or the address may need elevated privileges."
                )
            })?;

        info!("nsm listening on http://{bind_addr}");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("serve application")?;

        info!("nsm stopped");
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM so Docker can shut the process down cleanly.
async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!("failed to install SIGTERM handler: {error}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl+C, shutting down"),
        _ = terminate => info!("received SIGTERM, shutting down"),
    }
}
