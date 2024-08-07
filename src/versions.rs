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

use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Deserialize)]
struct CargoLock {
    package: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlanToml {
    #[serde(rename = "crate")]
    pub crates: Vec<Crate>,
}

#[derive(Debug, Deserialize)]
pub struct Crate {
    pub name: String,
    pub to: String,
    pub from: String,
    pub publish: Option<bool>,
}

// pub async fn update_orml_crates_version(
//     version: &str,
//     dep_table: &mut toml_edit::Table,
//     crates_versions: &BTreeMap<String, String>,
//     overwrite: bool
// ) -> Result<(), Box<dyn std::error::Error>> {

// }

pub async fn get_orml_crates_and_version(
    base_url: &str,
    version: &str,
) -> Result<Option<(Vec<String>, String)>, Box<dyn std::error::Error>> {
    if get_release_branches_versions(Repository::Orml)
        .await?
        .contains(&version.to_string())
    {
        let version_url = format!(
            "{}/open-web3-stack/open-runtime-module-library/polkadot-v{}/Cargo.dev.toml",
            base_url, version
        );
        let response = reqwest::Client::new()
            .get(&version_url)
            .header("User-Agent", "reqwest")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        let content = response.text().await?;

        Ok(Some(parse_orml_workspace_members(&content)))
    } else {
        log::error!(
            "No matching ORML release version found for corresponding polkadot-sdk version."
        );
        Ok(None)
    }
}

fn parse_orml_workspace_members(toml_content: &str) -> (Vec<String>, String) {
    let mut members = Vec::new();
    let mut crates_version = String::new();
    let mut in_workspace_members = false;

    for line in toml_content.lines() {
        if line.trim() == "[workspace]" {
            in_workspace_members = true;
            continue;
        }

        if line.trim().starts_with("# crates-version = \"") {
            crates_version = line
                .trim()
                .replace("# crates-version = \"", "")
                .trim_matches('"')
                .to_string();
            break;
        }

        if in_workspace_members {
            if line.trim().starts_with("members = [") {
                continue;
            } else if line.trim().ends_with(']') {
                in_workspace_members = false;
            } else if line.contains('/') {
                continue;
            } else {
                let member = line.trim().trim_matches(',').trim_matches('"');
                members.push(format!("orml-{}", member));
            }
        }
    }

    (members, crates_version)
}

pub fn include_orml_crates_in_version_mapping(
    crates_versions: &mut BTreeMap<String, String>,
    orml_crates_version: Option<(Vec<String>, String)>,
) {
    if let Some((orml_crates, orml_version)) = orml_crates_version {
        for crate_name in orml_crates {
            crates_versions.insert(crate_name, orml_version.clone());
        }
    }
}

pub async fn get_version_mapping_with_fallback(
    base_url: &str,
    version: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let result = get_version_mapping(base_url, version, "Plan.toml").await;

    if result.is_err() {
        println!("Failed to get Plan.toml, falling back to Cargo.lock.");
        get_version_mapping(base_url, version, "Cargo.lock").await
    } else {
        result
    }
}

fn version_to_url(
    base_url: &str,
    version: &str,
    source: &str,
) -> String {
    let version = if version.starts_with("stable") {
        version.into()
    } else {
        format!("release-crates-io-v{}", version)
    };

    format!("{}/paritytech/polkadot-sdk/{}/{}", base_url, version, source)
}

pub async fn get_version_mapping(
    base_url: &str,
    version: &str,
    source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let url = version_to_url(base_url, version, source);
    let response = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "reqwest")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;
    let content = response.text().await?;

    match source {
        "Cargo.lock" => get_cargo_packages(&content),
        "Plan.toml" => get_plan_packages(&content).await,
        _ => panic!("Unknown source: {}", source),
    }
}

fn get_cargo_packages(
    content: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let cargo_lock: CargoLock = toml::from_str(content)?;

    // Filter local packages and collect them into a JSON object
    let cargo_packages: BTreeMap<_, _> = cargo_lock
        .package
        .into_iter()
        .filter(|pkg| pkg.source.is_none())
        .map(|pkg| (pkg.name, pkg.version))
        .collect();

    Ok(cargo_packages)
}

async fn get_plan_packages(
    content: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let plan_toml: PlanToml = toml::from_str(content)?;

    let parity_owned_crates = get_parity_crate_owner_crates().await?;

    // Filter local packages and collect them into a JSON object
    let plan_packages: BTreeMap<_, _> = plan_toml
        .crates
        .into_iter()
        .filter(|pkg| {
            pkg.publish.unwrap_or(true) || {
                let placeholder = pkg.to == "0.0.0" && pkg.from == "0.0.0";
                let public_not_in_release = parity_owned_crates.contains(&pkg.name) && !placeholder;
                if public_not_in_release {
                    log::info!(
                        "Adding public crate not in release {}: {} -> {}",
                        pkg.name,
                        pkg.from,
                        pkg.to
                    );
                }
                public_not_in_release
            }
        })
        .map(|pkg| (pkg.name, pkg.to))
        .collect();

    Ok(plan_packages)
}

#[derive(serde::Deserialize, Debug)]
struct Branch {
    name: String,
}

#[derive(Default)]
struct RepositoryInfo {
    branches_url: String,
    gh_cmd_url: String,
    version_filter_string: String,
    version_replace_string: String,
}

pub enum Repository {
    /// The official ORML repository
    Orml,
    /// The official Polkadot SDK repository
    Psdk,
}

fn get_repository_info(repository: &Repository) -> RepositoryInfo {
    match repository {
        Repository::Orml => RepositoryInfo {
            branches_url: "https://api.github.com/repos/open-web3-stack/open-runtime-module-library/branches?per_page=100&page=".into(),
            gh_cmd_url: "/repos/open-web3-stack/open-runtime-module-library/branches?per_page=100&page=".into(),
            version_filter_string: "polkadot-v1".into(),
            version_replace_string: "polkadot-v".into()
        },
        Repository::Psdk => RepositoryInfo {
            branches_url: "https://api.github.com/repos/paritytech/polkadot-sdk/branches?per_page=100&page=".into(),
            gh_cmd_url: "/repos/paritytech/polkadot-sdk/branches/per_page=100&page={}".into(),
            version_filter_string: "release-crates-io-v".into(),
            version_replace_string: "release-crates-io-v".into()
        },
    }
}

pub async fn get_release_branches_versions(repository: Repository) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut release_branches = vec![];
    let repository_info = get_repository_info(&repository);

    for page in 1..100 {
        // currently there's 5 pages, so 100 should be enough
        let response = reqwest::Client::new()
            .get(format!("{}{}", repository_info.branches_url, page))
            .header("User-Agent", "reqwest")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        let output = if response.status().is_success() {
            response.text().await?
        } else {
            // query the github api using gh command
            String::from_utf8(
                std::process::Command::new("gh")
                    .args([
                        "api",
                        "-H",
                        "Accept: application/vnd.github+json",
                        "-H",
                        "X-GitHub-Api-Version: 2022-11-28",
                        &format!(
                            "{}{}",
                            repository_info.gh_cmd_url, page
                        ),
                    ])
                    .output()?
                    .stdout,
            )?
        };

        let branches: Vec<Branch> = serde_json::from_str(&output)?;

        let version_branches = branches
            .iter()
            .filter(|b| b.name.starts_with(&repository_info.version_filter_string))
            .filter(|b| (b.name != "polkadot-v1.0.0"))
            .map(|branch| branch.name.replace(&repository_info.version_replace_string, ""));

        release_branches = release_branches
            .into_iter()
            .chain(version_branches)
            .collect();

        if branches.len() < 100 {
            break;
        }
    }

    Ok(release_branches)
}

pub async fn get_parity_crate_owner_crates() -> Result<HashSet<String>, Box<dyn std::error::Error>>
{
    let mut parity_crates = HashSet::new();

    for page in 1..=10 {
        // Currently there are 7 pages (so this at most 1s)
        let response = reqwest::Client::new()
            .get(format!(
                "https://crates.io/api/v1/crates?page={}&per_page=100&user_id=150167", // parity-crate-owner
                page
            ))
            .header("User-Agent", "reqwest")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        let output = response.text().await?;

        let crates_data: serde_json::Value = serde_json::from_str(&output)?;

        let crates = crates_data["crates"]
            .as_array()
            .unwrap()
            .iter();

        let crates_len = crates.len();

        let crate_names = crates
            .filter(|crate_data| crate_data["max_version"].as_str().unwrap_or_default() != "0.0.0")
            .map(|crate_data| crate_data["id"].as_str().unwrap_or_default().to_string());

        parity_crates.extend(crate_names);

        if crates_len < 100 {
            break;
        }
    }

    Ok(parity_crates)
}
