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
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use toml_edit::DocumentMut;
use versions::{get_branch_mapping, get_version_mapping};

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

    /// Specifies the Polkadot SDK version.
    #[clap(
        short,
        long,
        conflicts_with = "branch",
        required_unless_present = "branch"
    )]
    version: Option<String>,

    /// Specifies a Polkadot SDK branch to get the versions. Can't be used at the same time as `version`.
    #[clap(short, long)]
    branch: Option<String>,

    /// Specifies the source file to get the versions.
    #[clap(short, long, value_parser = ["Cargo.lock", "Plan.toml"], default_value = "Plan.toml")]
    source: String,

    /// Overwrite local dependencies (using path).
    #[clap(short, long)]
    overwrite: bool,

    /// Specifies the git server to get the versions when using the `branch` flag.
    #[clap(short, long, default_value = DEFAULT_GIT_SERVER)]
    git_server: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cmd = Command::parse();

    let cargo_toml_path = validate_workspace_path(cmd.path)?;

    // Decide which branch data to use based on the branch name
    let crates_versions: BTreeMap<String, String> = if let Some(version) = cmd.version {
        serde_json::from_str(get_version_mapping(&version))?
    } else if let Some(branch) = cmd.branch {
        get_branch_mapping(&cmd.git_server, &branch, &cmd.source).await?
    } else {
        return Err("Please specify only a version or a branch".into());
    };

    update_dependencies(&cargo_toml_path, &crates_versions, cmd.overwrite)?;

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
) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml = update_dependencies_impl(cargo_toml_path, crates_versions, overwrite)?;

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
        Ok(Some(new_content))
    } else {
        Ok(None)
    }
}

fn update_table_dependencies(
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

            table.remove("git");
            table.remove("rev");
            table.remove("branch");
            table.remove("tag");
            table.remove("path");

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
