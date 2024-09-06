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

use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};

/// Represents the structure of a Cargo.lock file, including all packages.
#[derive(Debug, Deserialize)]
struct CargoLock {
    /// A list of packages included in the Cargo.lock file.
    package: Vec<Package>,
}

/// Represents a single package within a Cargo.lock file.
#[derive(Debug, Deserialize)]
struct Package {
    /// The name of the package.
    name: String,
    /// The version of the package.
    version: String,
    /// The source from which the package was retrieved(usually GitHub), if any.
    source: Option<String>,
}

/// Represents the structure of a Plan.toml file, with all crates.
#[derive(Debug, Deserialize)]
pub struct PlanToml {
    /// A list of crates included in the Plan.toml file.
    #[serde(rename = "crate")]
    pub crates: Vec<Crate>,
}

/// Represents a single crate within a Plan.toml file.
#[derive(Debug, Deserialize)]
pub struct Crate {
    /// The name of the crate.
    pub name: String,
    /// The version the crate is updating to.
    pub to: String,
    /// The current version of the crate.
    pub from: String,
    /// Indicates if the crate should be published.
    pub publish: Option<bool>,
}

/// Represents the structure of an Orml.toml file with workspace information.
#[derive(Debug, Deserialize)]
pub struct OrmlToml {
    /// The workspace information.
    pub workspace: Workspace,
}

/// Represents the metadata section within a workspace.
#[derive(Deserialize, Debug)]
pub struct Metadata {
    /// ORML specific metadata.
    orml: Orml,
}

/// Represents ORML specific metadata.
#[derive(Deserialize, Debug)]
pub struct Orml {
    /// The version of the crates managed by ORML.
    #[serde(rename = "crates-version")]
    crates_version: String,
}

/// Represents a workspace, including its members and metadata.
#[derive(Deserialize, Debug)]
pub struct Workspace {
    /// A list of members (crates) in the workspace.
    members: Vec<String>,
    /// Metadata associated with the workspace.
    metadata: Metadata,
}

/// Represents a tag by its name.
#[derive(Deserialize, Debug)]
pub struct TagInfo {
    /// The name of the tag.
    pub name: String,
}

const POLKADOT_SDK_TAGS_URL: &str =
    "https://api.github.com/repos/paritytech/polkadot-sdk/tags?per_page=100&page=";
const POLKADOT_SDK_TAGS_GH_CMD_URL: &str = "/repos/paritytech/polkadot-sdk/tags?per_page=100&page=";
const POLKADOT_SDK_STABLE_TAGS_REGEX: &str = r"^polkadot-stable\d+(-\d+)?$";

/// Fetches a combined list of Polkadot SDK release versions and stable tag releases.
///
/// This function first retrieves release branch versions from the Polkadot SDK and
/// then fetches stable tag releases versions. It combines these two lists into a
/// single list of version strings.
///
/// # Returns
/// A `Result` containing either a `Vec<String>` of combined version names on success,
/// or an `Error` if any part of the process fails.
///
/// # Errors
/// This function can return an error if either the fetching of release branches versions
/// or the fetching of stable tag versions encounters an issue.
pub async fn get_polkadot_sdk_versions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut crates_io_releases = get_release_branches_versions(Repository::Psdk).await?;
    let mut stable_tag_versions = get_stable_tag_versions().await?;
    crates_io_releases.append(&mut stable_tag_versions);
    Ok(crates_io_releases)
}

/// Fetches a list of stable tag versions for the Polkadot SDK from GitHub.
///
/// This function queries GitHub's API to retrieve tags for the Polkadot SDK,
/// filtering them based on a predefined regex to identify stable versions.
/// If the direct API request fails, it falls back to using the GitHub CLI.
///
/// # Returns
/// A `Result` containing either a `Vec<String>` of stable tag names on success,
/// or an `Error` if any part of the process fails.
///
/// # Errors
/// This function can return an error if the HTTP request fails, if parsing the
/// response into text fails, if executing the GitHub CLI command fails, or if
/// parsing the JSON response into `Vec<TagInfo>` fails.
pub async fn get_stable_tag_versions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut release_tags = vec![];

    for page in 1..100 {
        let response = reqwest::Client::new()
            .get(format!("{}{}", POLKADOT_SDK_TAGS_URL, page))
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
                        &format!("{}{}", POLKADOT_SDK_TAGS_GH_CMD_URL, page),
                    ])
                    .output()?
                    .stdout,
            )?
        };

        let tag_branches: Vec<TagInfo> = serde_json::from_str(&output)?;
        let tag_regex = Regex::new(POLKADOT_SDK_STABLE_TAGS_REGEX).unwrap();

        let stable_tag_branches = tag_branches
            .iter()
            .filter(|b| tag_regex.is_match(&b.name))
            .map(|branch| branch.name.to_string());

        release_tags = release_tags
            .into_iter()
            .chain(stable_tag_branches)
            .collect();

        if tag_branches.len() < 100 {
            break;
        }
    }

    Ok(release_tags)
}

/// Fetches the ORML crates and their versions for a specific version of Polkadot.
///
/// This function queries a repository for a specific version of the ORML crates,
/// attempting to retrieve the `Cargo.dev.toml` file that lists the ORML workspace members
/// and the corresponding crates version. It uses the provided `base_url` and `version` to
/// construct the URL for the request.
///
/// # Arguments
///
/// * `base_url` - The base URL of GitHub.
/// * `version` - The release version of the Polkadot-sdk for which ORML crates' versions are being fetched.
///
/// # Returns
///
/// Returns `Ok(Some(OrmlToml))` if the `Cargo.dev.toml` file is successfully retrieved and parsed,
/// indicating the ORML crates and their versions. Returns `Ok(None)` if no matching ORML release
/// version is found for the corresponding Polkadot version. In case of any error during the
/// fetching or parsing process, an error is returned.
///
/// # Errors
///
/// This function returns an error if there is any issue with the HTTP request, response parsing,
/// or if the required fields are not found in the `Cargo.dev.toml` file.
///
/// # Examples
///
/// ```
/// #[tokio::main]
/// async fn main() {
///     let base_url = "https://raw.githubusercontent.com";
///     let version = "1.12.0";
///     match get_orml_crates_and_version(base_url, version).await {
///         Ok(Some(orml_toml)) => println!("ORML crates: {:?}", orml_toml),
///         Ok(None) => println!("No matching ORML version found."),
///         Err(e) => println!("Error fetching ORML crates: {}", e),
///     }
/// }
/// ```
pub async fn get_orml_crates_and_version(
    base_url: &str,
    version: &str,
) -> Result<Option<OrmlToml>, Box<dyn std::error::Error>> {
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

        let orml_workspace_members = toml::from_str::<OrmlToml>(&content)
            .map_err(|_| "Error Parsing ORML TOML. Required Fields not Found")?;
        Ok(Some(orml_workspace_members))
    } else {
        log::error!(
            "No matching ORML release version found for corresponding polkadot-sdk version."
        );
        Ok(None)
    }
}

/// Includes ORML crates in the version mapping.
///
/// This function updates a given version mapping (`BTreeMap`) by adding the versions of ORML
/// crates obtained from a `OrmlToml` instance. It prefixes each crate name with "orml-" and
/// inserts the corresponding version into the map. If the `orml_crates_version` is `None`,
/// the function does nothing.
///
/// # Arguments
///
/// * `crates_versions` - A mutable reference to a `BTreeMap` where the original polkadot-sdk
///   crate names and versions are stored.
/// * `orml_crates_version` - An `Option<OrmlToml>` that may contain the ORML crates and their
///   versions.
///
/// # Examples
///
/// ```
/// let mut version_map: BTreeMap<String, String> = BTreeMap::new();
/// include_orml_crates_in_version_mapping(&mut version_map, Some(orml_toml));
/// ```
pub fn include_orml_crates_in_version_mapping(
    crates_versions: &mut BTreeMap<String, String>,
    orml_crates_version: Option<OrmlToml>,
) {
    if let Some(orml_toml) = orml_crates_version {
        for crate_name in orml_toml.workspace.members {
            crates_versions.insert(
                format!("orml-{}", crate_name),
                orml_toml.workspace.metadata.orml.crates_version.clone(),
            );
        }
    }
}

pub async fn get_version_mapping_with_fallback(
    base_url: &str,
    version: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let result = get_version_mapping(base_url, version, "Plan.toml").await;

    match result {
        Err(_) => get_version_mapping(base_url, version, "Cargo.lock").await,
        Ok(_) => result,
    }
}

fn version_to_url(base_url: &str, version: &str, source: &str) -> String {
    let stable_tag_regex_patten = Regex::new(POLKADOT_SDK_STABLE_TAGS_REGEX).unwrap();
    let version = if version.starts_with("stable") {
        format!("polkadot-{}", version)
    } else if stable_tag_regex_patten.is_match(version) {
        version.into()
    } else {
        format!("release-crates-io-v{}", version)
    };

    format!(
        "{}/paritytech/polkadot-sdk/{}/{}",
        base_url, version, source
    )
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

    let content = match response.error_for_status() {
        Ok(response) => response.text().await?,
        Err(err) => return Err(err.into()),
    };

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

/// Represents a single branch in a repository.
///
/// This struct is used to deserialize JSON data from a repository's branch list.
#[derive(serde::Deserialize, Debug)]
struct Branch {
    /// The name of the branch.
    name: String,
}

/// Contains information about a repository.
///
/// This struct holds various URLs and strings used to interact with a repository,
/// including fetching branches and processing version information.
struct RepositoryInfo {
    /// The URL to fetch branch information from the repository.
    branches_url: String,
    /// The URL for GitHub commands related to the repository.
    gh_cmd_url: String,
    /// A string used to filter versions from branch names.
    version_filter_string: String,
    /// A string used to replace parts of the version string if necessary.
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
            gh_cmd_url: "/repos/paritytech/polkadot-sdk/branches?per_page=100&page=".into(),
            version_filter_string: "release-crates-io-v".into(),
            version_replace_string: "release-crates-io-v".into()
        },
    }
}

/// Fetches the versions of release branches from a repository.
///
/// This asynchronous function queries a repository for its branches and filters out those
/// that match a specific versioning pattern. It supports fetching data via HTTP requests
/// and, in case of failure, falls back to querying the GitHub API using the `gh` command-line tool.
///
/// # Arguments
///
/// * `repository` - A `Repository` enum specifying whether to query the ORML or Polkadot SDK repository.
///
/// # Returns
///
/// Returns a `Result` containing either a vector of version strings on success or an error on failure.
///
/// # Errors
///
/// This function can return an error in several cases, including but not limited to:
/// - Network failures during the HTTP request.
/// - JSON parsing errors when deserializing the response into `Branch` structs.
/// - UTF-8 decoding errors when processing the output of the `gh` command.
/// - I/O errors when executing the `gh` command.
///
/// # Examples
///
/// ```no_run
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let orml_repository = Repository::Orml;
///     let orml_versions = get_release_branches_versions(orml_repository).await?;
///     println!("Orml Release versions: {:?}", orml_versions);
///
///     let psdk_repository = Repository::Psdk;
///     let psdk_versions = get_release_branches_versions(psdk_repository).await?;
///     println!("Polkadot-sdk Release versions: {:?}", psdk_versions);
///
///     Ok(())
/// }
/// ```
pub async fn get_release_branches_versions(
    repository: Repository,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
                        &format!("{}{}", repository_info.gh_cmd_url, page),
                    ])
                    .output()?
                    .stdout,
            )?
        };

        let branches: Vec<Branch> = serde_json::from_str(&output)?;

        let version_branches = branches
            .iter()
            .filter(|b| b.name.starts_with(&repository_info.version_filter_string))
            .filter(|b| (b.name != "polkadot-v1.0.0")) // This is in place to filter that particular orml version as it is not a valid polkadot-sdk release version
            .map(|branch| {
                branch
                    .name
                    .replace(&repository_info.version_replace_string, "")
            });

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

        let crates = crates_data["crates"].as_array().unwrap().iter();

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
