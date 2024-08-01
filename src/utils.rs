use std::path::PathBuf;

pub fn rename_deps(dep_value: &mut toml_edit::Item, dep_key_str: &str) -> String {
    let lookup_key = if let Some(table) = dep_value.as_table_like() {
        table
            .get("package")
            .and_then(|p| p.as_str())
            .unwrap_or(dep_key_str)
    } else {
        dep_key_str
    };
    lookup_key.into()
}

pub fn remove_keys_from_table(table: &mut dyn toml_edit::TableLike) {
    table.remove("git");
    table.remove("rev");
    table.remove("branch");
    table.remove("tag");
    table.remove("path");
}

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
