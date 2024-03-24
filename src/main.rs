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

mod versions;
mod tests;

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use clap::{App, Arg};
use serde_json::from_str;
use std::collections::BTreeMap;
use toml_edit::Document;
use versions::get_version_mapping;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Polkadot SDK Version Manager")
        .version("1.0")
        .about("Updates Cargo.toml dependencies based on Polkadot SDK crates.io release branch")
        .author("Patricio (patriciobcs)")
        .arg(Arg::with_name("PATH")
             .help("Path to the workspace root folder or workspace Cargo.toml.")
             .default_value("Cargo.toml")
             .short('p')
             .long("path"))
        .arg(Arg::with_name("VERSION")
             .help("Specifies the Polkadot SDK version")
             .default_value("1.3.0")
             .short('v')
             .long("version"))
        .arg(Arg::with_name("OVERWRITE")
             .help("Overwrite local dependencies (using path)")
             .takes_value(false)
             .short('o')
             .long("overwrite"))
        .get_matches();

    let cargo_toml_path = validate_workspace_path(matches.value_of("PATH").unwrap())?;
    let version = matches.value_of("VERSION").unwrap();
    let overwrite = matches.is_present("OVERWRITE");

    // Decide which branch data to use based on the branch name
    let crates_versions_data = get_version_mapping(version);

    let crates_versions: BTreeMap<String, String> = from_str(crates_versions_data)?;

    update_dependencies(&cargo_toml_path, &crates_versions, overwrite)?;

    Ok(())
}

fn validate_workspace_path(cargo_toml_path: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = PathBuf::from(cargo_toml_path);
    if path.is_dir() {
        path = path.join("Cargo.toml");
    }

    if !path.exists() {
        return Err(format!("Could not find workspace root Cargo.toml file at {}", path.display()).into());
    }

    Ok(path)
}

fn update_dependencies(cargo_toml_path: &Path, crates_versions: &BTreeMap<String, String>, overwrite: bool) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml = update_dependencies_impl(cargo_toml_path, crates_versions, overwrite)?;

    fs::write(cargo_toml_path, cargo_toml)?;
    println!("Updated dependencies in {}", cargo_toml_path.display());
    Ok(())
}

fn update_dependencies_impl(cargo_toml_path: &Path, crates_versions: &BTreeMap<String, String>, overwrite: bool) -> Result<String, Box<dyn std::error::Error>> {
    let cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
    let mut cargo_toml: Document = cargo_toml_content.parse()?;
    // Check if cargo workspace is defined
    let deps = match cargo_toml.as_table_mut().get_mut("workspace") {
        Some(toml_edit::Item::Table(table)) => table,
        _ => cargo_toml.as_table_mut()
    };

    for table in ["dependencies", "dev-dependencies", "build-dependencies"].iter() {
        if let Some(toml_edit::Item::Table(dep_table)) = deps.get_mut(table) {
                update_table_dependencies(dep_table, crates_versions, overwrite);
        }
    }

    Ok(cargo_toml.to_string())
}

fn update_table_dependencies(dep_table: &mut toml_edit::Table, crates_versions: &BTreeMap<String, String>, overwrite: bool) {
    for (dep_key, dep_value) in dep_table.iter_mut() {
        let dep_key_str = dep_key.get();

        let lookup_key = if let Some(table) = dep_value.as_table_like() {
            table.get("package")
                 .and_then(|p| p.as_str())
                 .unwrap_or(dep_key_str)
        } else {
            dep_key_str
        };

        if let Some(crate_version) = crates_versions.get(lookup_key) {
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
                new_table.get_or_insert("version", toml_edit::value(crate_version.clone()).as_value().unwrap());
                
                for (key, value) in table.iter() {
                    // Ensure we're inserting `Value`s, not `Item`s
                    if key != "version" && value.is_value() {
                        new_table.get_or_insert(key, value.as_value().unwrap().clone());
                    }
                }
                new_table.fmt();
                
                // Replace the original table-like item with the new inline table
                *dep_value = toml_edit::Item::Value(toml_edit::Value::InlineTable(new_table));
            }
        }
    }
}
