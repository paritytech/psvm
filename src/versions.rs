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

pub fn get_version_mapping(version: &str) -> &'static str {
    match version {
        "1.3.0" => include_str!("versions/release-crates-io-v1.3.0.json"),
        "1.4.0" => include_str!("versions/release-crates-io-v1.4.0.json"),
        "1.5.0" => include_str!("versions/release-crates-io-v1.5.0.json"),
        "1.6.0" => include_str!("versions/release-crates-io-v1.6.0.json"),
        "1.7.0" => include_str!("versions/release-crates-io-v1.7.0.json"),
        "1.8.0" => include_str!("versions/release-crates-io-v1.8.0.json"),
        "1.9.0" => include_str!("versions/release-crates-io-v1.9.0.json"),
        _ => panic!("Version {} not available", version),
    }
}

pub async fn get_branch_mapping(
    base_url: &str,
    branch: &str,
    source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let url = format!("{}/paritytech/polkadot-sdk/{}/{}", base_url, branch, source);
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
        .filter(|pkg| pkg.publish.is_none())
        .map(|pkg| (pkg.name, pkg.to))
        .collect();

    Ok(plan_packages)
}
