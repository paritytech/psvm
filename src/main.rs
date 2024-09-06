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

mod tests;
mod versions;

use clap::Parser;
use env_logger::Env;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;
use versions::{
    get_orml_crates_and_version, get_polkadot_sdk_versions, get_release_branches_versions,
    get_version_mapping_with_fallback, include_orml_crates_in_version_mapping, Repository,
};

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

fn validate_workspace_path(mut path: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_dir() {
        path = path.join("Cargo.toml");
    }

    if !path.exists() {
        return Err(format!(
            "Could not find workspace root Cargo.toml file at {}",
            path.display()
        )
        .into());
    }

    Ok(path)
}

fn update_dependencies(
    cargo_toml_path: &Path,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
    only_check: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml =
        update_dependencies_impl(cargo_toml_path, crates_versions, overwrite, only_check)?;

    match cargo_toml {
        Some(new_content) => {
            fs::write(cargo_toml_path, new_content)?;
            println!("Updated dependencies in {}", cargo_toml_path.display());
        }
        None => {
            println!(
                "Dependencies in {} are already up to date",
                cargo_toml_path.display()
            );
        }
    }

    Ok(())
}

fn update_dependencies_impl(
    cargo_toml_path: &Path,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
    only_check: bool,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
    let mut cargo_toml: DocumentMut = cargo_toml_content.parse()?;
    // Check if cargo workspace is defined
    let deps = match cargo_toml.as_table_mut().get_mut("workspace") {
        Some(toml_edit::Item::Table(table)) => table,
        _ => cargo_toml.as_table_mut(),
    };

    for table in ["dependencies", "dev-dependencies", "build-dependencies"].iter() {
        if let Some(toml_edit::Item::Table(dep_table)) = deps.get_mut(table) {
            update_table_dependencies(dep_table, crates_versions, overwrite);
        }
    }

    let new_content = cargo_toml.to_string();
    if new_content != cargo_toml_content {
        if only_check {
            Err("Dependencies are not up to date".into())
        } else {
            Ok(Some(new_content))
        }
    } else {
        Ok(None)
    }
}

pub fn update_table_dependencies(
    dep_table: &mut toml_edit::Table,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
) {
    for (dep_key, dep_value) in dep_table.iter_mut() {
        let dep_key_str = dep_key.get();

        // account for dep renaming:
        let lookup_key = if let Some(table) = dep_value.as_table_like() {
            table
                .get("package")
                .and_then(|p| p.as_str())
                .unwrap_or(dep_key_str)
        } else {
            dep_key_str
        };

        let Some(crate_version) = crates_versions.get(lookup_key) else {
            log::debug!("Could not find version for {}", lookup_key);
            continue;
        };

        if let Some(table) = dep_value.as_table_like_mut() {
            if !overwrite && table.get("path").is_some() {
                continue;
            }

            table.remove("rev");
            table.remove("branch");
            table.remove("tag");
            table.remove("path");
            table.remove("git");

            let mut new_table = toml_edit::InlineTable::default();

            // Directly create a `toml_edit::Value` for the version
            new_table.get_or_insert(
                "version",
                toml_edit::value(crate_version.clone()).as_value().unwrap(),
            );

            for (key, value) in table.iter() {
                // Ensure we're inserting `Value`s, not `Item`s
                if key != "version" && value.is_value() {
                    new_table.get_or_insert(key, value.as_value().unwrap().clone());
                }
            }
            new_table.fmt();

            // Replace the original table-like item with the new inline table
            *dep_value = toml_edit::Item::Value(toml_edit::Value::InlineTable(new_table));
        } else if dep_value.is_str() {
            *dep_value = toml_edit::value(crate_version.clone());
        } else {
            log::error!("Unexpected dependency value type for {}", dep_key_str);
            continue;
        }

        log::debug!("Setting {} to {}", dep_key_str, crate_version);
    }
}
