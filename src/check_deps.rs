use crate::utils::rename_deps;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;
pub fn check_dependencies(
    cargo_toml_path: &Path,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    check_dependencies_impl(cargo_toml_path, crates_versions, overwrite)?;
    log::info!(
        "Checked for dependencies in {}. All Up-to-date!",
        cargo_toml_path.display()
    );

    Ok(())
}

pub fn check_dependencies_impl(
    cargo_toml_path: &Path,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_toml_content = fs::read_to_string(cargo_toml_path)?;
    let mut cargo_toml: DocumentMut = local_toml_content.parse()?;
    let mut has_mismatch = false;

    // Check if cargo workspace is defined
    let deps = match cargo_toml.as_table_mut().get_mut("workspace") {
        Some(toml_edit::Item::Table(table)) => table,
        _ => cargo_toml.as_table_mut(),
    };

    for table in ["dependencies", "dev-dependencies", "build-dependencies"].iter() {
        if let Some(toml_edit::Item::Table(dep_table)) = deps.get_mut(table) {
            has_mismatch = check_table_dependencies(dep_table, crates_versions, overwrite)?;
        }
    }

    if has_mismatch {
        return Err("One or More Dependency version mismatch found".into());
    }

    Ok(())
}

fn check_table_dependencies(
    dep_table: &mut toml_edit::Table,
    crates_versions: &BTreeMap<String, String>,
    overwrite: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut local_toml_version: &str;
    let mut has_mismatch = false;
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

            let local_toml_version_option = table.get("version");
            if local_toml_version_option.is_none() {
                log::error!("Dependency format invalid for {}", dep_key_str);
                return Err("Invalid Dependency Format".into());
            } else {
                local_toml_version = local_toml_version_option.unwrap().as_str().unwrap();
            }
        } else if dep_value.is_str() {
            local_toml_version = dep_value.as_str().unwrap();
        } else {
            log::error!("Unexpected dependency value type for {}", dep_key_str);
            continue;
        }

        if local_toml_version != crate_version {
            has_mismatch = true;
            log::error!(
                "Dependency version mismatch for {} in Cargo.toml. Expected: {}, Found: {}",
                dep_key_str,
                crate_version,
                local_toml_version
            );
        }
    }

    Ok(has_mismatch)
}
