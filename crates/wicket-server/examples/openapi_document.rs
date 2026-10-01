// SPDX-License-Identifier: AGPL-3.0-or-later

//! Write or check the OpenAPI document `GET /api/v1/openapi.json` serves.
//!
//! Loads configuration the way `wicket serve` does ([`wicket_server::Config::load`]),
//! boots [`wicket_server::App`], and calls [`wicket_server::openapi_document`].
//! Regeneration is `just openapi-document`. `--check` compares JSON values, not bytes:
//! the HTTP encoder is compact and the fixture is pretty.

#![allow(unused_crate_dependencies)] // the example links the package graph and calls the library.

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::Parser;
use serde_json::Value;

/// Arguments for the document writer.
#[derive(Parser, Debug)]
#[command(
    name = "openapi_document",
    about = "Write or check the OpenAPI document the HTTP handler serves"
)]
struct Args {
    /// Installation profile: `plain-shop` or `regulated-device`.
    #[arg(long, value_parser = ["plain-shop", "regulated-device"])]
    profile: String,
    /// Fixture path to write, or to compare when `--check` is set.
    #[arg(long)]
    out: PathBuf,
    /// Exit 0 only when the booted document equals the JSON at `--out`.
    #[arg(long)]
    check: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let cfg = wicket_server::Config::load(Some(&args.profile), None, None, None)
        .context("load config")?;
    let app = wicket_server::App::boot(cfg).await.context("boot")?;
    let served = wicket_server::openapi_document(&app.state());

    if args.check {
        let file = File::open(&args.out).with_context(|| format!("open {}", args.out.display()))?;
        let committed: Value = serde_json::from_reader(BufReader::new(file))
            .with_context(|| format!("parse {}", args.out.display()))?;
        if committed != served {
            bail!("served document differs from {}", args.out.display());
        }
        println!(
            "openapi_document: {} matches the served document ({})",
            args.out.display(),
            args.profile
        );
        return Ok(());
    }

    if let Some(parent) = args.out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut file =
        File::create(&args.out).with_context(|| format!("create {}", args.out.display()))?;
    serde_json::to_writer_pretty(&mut file, &served)?;
    file.write_all(b"\n")?;
    println!(
        "openapi_document: wrote {} ({})",
        args.out.display(),
        args.profile
    );
    Ok(())
}
