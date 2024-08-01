use crate::utils::{remove_keys_from_table, rename_deps};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

pub fn update_dependencies(
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

pub fn update_dependencies_impl(
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
        let lookup_key = rename_deps(dep_value, dep_key_str);

        let Some(crate_version) = crates_versions.get(&lookup_key) else {
            log::debug!("Could not find version for {}", lookup_key);
            continue;
        };

        if let Some(table) = dep_value.as_table_like_mut() {
            if !overwrite && table.get("path").is_some() {
                continue;
            }
            remove_keys_from_table(table);

            let mut new_table = toml_edit::InlineTable::default();

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
