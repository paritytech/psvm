// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod check_deps;
mod tests;
mod update_deps;
mod utils;
mod versions;

use check_deps::check_dependencies;
use clap::Parser;
use env_logger::Env;
use std::path::PathBuf;
use update_deps::update_dependencies;
use utils::validate_workspace_path;
use versions::get_release_branches_versions;
use versions::get_version_mapping_with_fallback;

pub const DEFAULT_GIT_SERVER: &str = "https://raw.githubusercontent.com";

/// Polkadot SDK Version Manager.
///
/// Updates Cargo.toml dependencies based on Polkadot SDK crates.io release branch.
#[derive(Parser, Debug)]
#[command(about, author)]
struct Command {
    /// Path to a crate folder or Cargo.toml file.
    #[clap(short, long, default_value = "Cargo.toml")]
    path: PathBuf,

    /// Specifies the Polkadot SDK version. Use '--list' flag to display available versions.
    #[clap(
        short,
        long,
        required_unless_present = "list",
        required_unless_present = "check"
    )]
    version: Option<String>,

    /// Checks if deps have the same version as the specified Polkadot SDK version.
    #[clap(
        short,
        long,
        required_unless_present = "version",
        required_unless_present = "list"
    )]
    check: Option<String>,

    /// Overwrite local dependencies (using path) with same name as the ones in the Polkadot SDK.
    #[clap(short, long)]
    overwrite: bool,

    /// List available versions.
    #[clap(short, long)]
    list: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cmd = Command::parse();

    match cmd {
        Command { list: true, .. } => {
            let crates_versions = get_release_branches_versions().await?;
            println!("Available versions:");
            for version in crates_versions.iter() {
                println!("- {}", version);
            }
        }
        Command {
            version: Some(version),
            ..
        } => {
            let cargo_toml_path = validate_workspace_path(cmd.path)?;
            let crates_versions =
                get_version_mapping_with_fallback(DEFAULT_GIT_SERVER, &version).await?;
            update_dependencies(&cargo_toml_path, &crates_versions, cmd.overwrite)?;
        }
        Command {
            check: Some(check_version),
            ..
        } => {
            let cargo_toml_path = validate_workspace_path(cmd.path)?;
            let crates_versions =
                get_version_mapping_with_fallback(DEFAULT_GIT_SERVER, &check_version).await?;
            // Here, you might want to check dependency versions without updating them
            check_dependencies(&cargo_toml_path, &crates_versions, false)?;
        }
        _ => {
            return Err("Invalid flag. Use '--help' to display available flags.".into());
        }
    }

    Ok(())
}
