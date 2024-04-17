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
use std::collections::BTreeMap;

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
struct PlanToml {
    #[serde(rename = "crate")]
    crates: Vec<Crate>,
}

#[derive(Debug, Deserialize)]
struct Crate {
    name: String,
    to: String,
    publish: Option<bool>,
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

pub async fn get_version_mapping(
    base_url: &str,
    version: &str,
    source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/paritytech/polkadot-sdk/release-crates-io-v{}/{}",
        base_url, version, source
    );
    let response = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "reqwest")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    let content = response.text().await?;

    match source {
        "Cargo.lock" => get_cargo_packages(&content),
        "Plan.toml" => get_plan_packages(&content),
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

fn get_plan_packages(
    content: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let plan_toml: PlanToml = toml::from_str(content)?;

    // Filter local packages and collect them into a JSON object
    let plan_packages: BTreeMap<_, _> = plan_toml
        .crates
        .into_iter()
        .filter(|pkg| pkg.publish.unwrap_or(true))
        .map(|pkg| (pkg.name, pkg.to))
        .collect();

    Ok(plan_packages)
}

#[derive(serde::Deserialize, Debug)]
struct Branch {
    name: String,
}

pub async fn get_release_branches_versions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut release_branches = vec![];

    for page in 1..100 {
        // currently there's 5 pages, so 100 should be enough
        let response = reqwest::Client::new()
        .get(format!("https://api.github.com/repos/paritytech/polkadot-sdk/branches?per_page=100&page={}", page))
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
                            "/repos/paritytech/polkadot-sdk/branches?per_page=100&page={}",
                            page
                        ),
                    ])
                    .output()?
                    .stdout,
            )?
        };

        let branches: Vec<Branch> = serde_json::from_str(&output)?;

        let version_branches = branches
            .iter()
            .filter(|b| b.name.starts_with("release-crates-io-v"))
            .map(|branch| branch.name.replace("release-crates-io-v", ""));

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
