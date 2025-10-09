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

use clap::Parser;
use env_logger::Env;
use psvm::{
    get_orml_crates_and_version, get_polkadot_sdk_versions, get_release_branches_versions,
    get_version_mapping_with_fallback, include_orml_crates_in_version_mapping, update_dependencies,
    validate_workspace_path, Repository, DEFAULT_GIT_SERVER,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

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
    #[clap(short, long, required_unless_present = "list")]
    version: Option<String>,

    /// Overwrite local dependencies (using path) with same name as the ones in the Polkadot SDK.
    #[clap(short, long)]
    overwrite: bool,

    /// List available versions.
    #[clap(short, long)]
    list: bool,

    /// Check if the dependencies versions match the Polkadot SDK version. Does not update the Cargo.toml
    #[clap(short, long)]
    check: bool,

    /// To either list available ORML versions or update the Cargo.toml file with corresponding ORML versions.
    #[clap(short('O'), long)]
    orml: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cmd = Command::parse();

    if cmd.list {
        let crates_versions = if cmd.orml {
            get_release_branches_versions(Repository::Orml).await?
        } else {
            get_polkadot_sdk_versions().await?
        };

        println!("Available versions:");
        for version in crates_versions.iter() {
            println!("- {}", version);
        }
        return Ok(());
    }

    let version = cmd.version.unwrap(); // Safe to unwrap due to `required_unless_present`

    let cargo_toml_path = validate_workspace_path(cmd.path)?;

    // Decide which branch data to use based on the branch name
    let mut crates_versions: BTreeMap<String, String> =
        get_version_mapping_with_fallback(DEFAULT_GIT_SERVER, &version).await?;

    if cmd.orml {
        let orml_crates = get_orml_crates_and_version(DEFAULT_GIT_SERVER, &version).await?;
        include_orml_crates_in_version_mapping(&mut crates_versions, orml_crates);
    }

    update_dependencies(&cargo_toml_path, &crates_versions, cmd.overwrite, cmd.check)?;

    Ok(())
}
