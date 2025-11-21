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

//! Polkadot SDK Version Manager Library
//!
//! This library provides functionality to manage and update Polkadot SDK dependencies
//! in Cargo.toml files.

mod tests;
pub mod versions;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

pub use versions::{
    get_orml_crates_and_version, get_polkadot_sdk_versions, get_release_branches_versions,
    get_version_mapping_with_fallback, include_orml_crates_in_version_mapping, Repository,
};

pub const DEFAULT_GIT_SERVER: &str = "https://raw.githubusercontent.com";

/// Validates that the provided path points to a valid Cargo.toml file.
///
/// If the path is a directory, it will append "Cargo.toml" to it.
/// Returns an error if the resulting path does not exist.
///
/// # Arguments
///
/// * `path` - A PathBuf that should point to either a Cargo.toml file or a directory containing one
///
/// # Errors
///
/// Returns an error if the Cargo.toml file cannot be found at the specified path.
pub fn validate_workspace_path(mut path: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
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

/// Updates dependencies in a Cargo.toml file based on the provided version mappings.
///
/// # Arguments
///
/// * `cargo_toml_path` - Path to the Cargo.toml file to update
/// * `crates_versions` - A map of crate names to their versions
/// * `overwrite` - If true, will overwrite local path dependencies
/// * `only_check` - If true, only checks if dependencies match without updating
///
/// # Returns
///
/// Returns `Ok(bool)` where:
/// - `true` indicates dependencies were updated.
/// - `false` indicates no updates were needed.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read or written
/// - The TOML content is invalid
/// - `only_check` is true and dependencies are not up to date
pub fn update_dependencies(
    cargo_toml_path: &Path,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
    only_check: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let cargo_toml =
        update_dependencies_impl(cargo_toml_path, crates_versions, overwrite, only_check)?;

    let updated = if let Some(new_content) = cargo_toml {
        fs::write(cargo_toml_path, new_content)?;
        true
    } else {
        false
    };

    Ok(updated)
}

/// Internal implementation of dependency update logic.
///
/// Returns `Some(String)` with the new content if changes were made,
/// or `None` if no changes were needed.
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

/// Updates dependencies within a specific TOML table.
///
/// This function modifies the dependency table in-place, updating versions
/// and removing git/path-related fields as appropriate.
///
/// # Arguments
///
/// * `dep_table` - The TOML table containing dependencies
/// * `crates_versions` - A map of crate names to their versions
/// * `overwrite` - If true, will overwrite local path dependencies
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
