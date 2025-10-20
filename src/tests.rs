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

#[cfg(test)]
mod tests {
    use crate::versions::get_orml_crates_and_version;
    use crate::versions::get_version_mapping_with_fallback;
    use crate::versions::include_orml_crates_in_version_mapping;
    use crate::versions::Repository;
    use std::{error::Error, path::Path};

    async fn verify_version_mapping(
        version: &str,
        input_cargo_toml_path: &Path,
        expected_cargo_toml: &str,
    ) {
        let crates_versions = get_version_mapping_with_fallback(crate::DEFAULT_GIT_SERVER, version)
            .await
            .unwrap();

        // Call the refactored logic function with the test data
        let result =
            crate::update_dependencies_impl(&input_cargo_toml_path, &crates_versions, false, false)
                .unwrap();

        // Assert that the result matches the expected output
        assert_eq!(result, Some(expected_cargo_toml.into()));
    }

    async fn verify_version_checking(
        version: &str,
        input_cargo_toml_path: &Path,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let mut crates_versions =
            get_version_mapping_with_fallback(crate::DEFAULT_GIT_SERVER, version)
                .await
                .unwrap();

        let orml_crates_version =
            get_orml_crates_and_version(crate::DEFAULT_GIT_SERVER, &version).await?;
        include_orml_crates_in_version_mapping(&mut crates_versions, orml_crates_version);

        // Call the refactored logic function with the test data
        let result =
            crate::update_dependencies_impl(&input_cargo_toml_path, &crates_versions, false, true);

        result
    }

    async fn verify_orml_version_mapping(
        version: &str,
        input_cargo_toml_path: &Path,
        expected_cargo_toml: &str,
    ) {
        let mut crates_versions =
            get_version_mapping_with_fallback(crate::DEFAULT_GIT_SERVER, version)
                .await
                .unwrap();

        let orml_crates_version = get_orml_crates_and_version(crate::DEFAULT_GIT_SERVER, &version)
            .await
            .unwrap();
        include_orml_crates_in_version_mapping(&mut crates_versions, orml_crates_version);

        // Call the refactored logic function with the test data
        let result =
            crate::update_dependencies_impl(&input_cargo_toml_path, &crates_versions, false, false)
                .unwrap();

        // Compare line-by-line so we can debug the first mismatch
        let result_lines = result.unwrap().lines().map(|line| line.to_string()).collect::<Vec<String>>();
        let expected_lines = expected_cargo_toml.lines().map(|line| line.to_string()).collect::<Vec<String>>();
        assert_eq!(result_lines.len(), expected_lines.len());
        for (result_line, expected_line) in result_lines.iter().zip(expected_lines.iter()) {
            assert_eq!(result_line, expected_line);
        }
    }

    #[tokio::test]
    // cargo psvm -v 1.3.0
    // This version doesn't have the Plan.toml file, so it will fallback to Cargo.lock
    async fn test_get_version_with_fallback() {
        let input_cargo_toml_path = Path::new("src/testing/cargo-lock/input.Cargo.toml");
        let expected_cargo_toml = include_str!("testing/cargo-lock/output.Cargo.toml");
        let version = "1.3.0";

        verify_version_mapping(version, input_cargo_toml_path, expected_cargo_toml).await;
    }

    #[tokio::test]
    // cargo psvm -v 1.14.0 -c
    // This version has the Plan.toml file, so it will not fallback to Cargo.lock
    // and check if the versions in the local toml file comply with the Plan.toml file
    async fn test_check_version_without_fallback_passes() {
        let input_cargo_toml_path = Path::new("src/testing/plan-toml/check.Cargo.toml");
        let version = "1.14.0";

        let res = verify_version_checking(version, input_cargo_toml_path).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[tokio::test]
    // cargo psvm -v 1.6.0 -c -O
    // This version has the Plan.toml file, so it will not fallback to Cargo.lock
    // and check if the versions in the local toml file comply with the Plan.toml file,
    // including the ORML crates.
    async fn test_check_version_with_orml_passes() {
        let input_cargo_toml_path = Path::new("src/testing/orml/output.Cargo.toml");
        let version = "1.6.0";

        let res = verify_version_checking(version, input_cargo_toml_path).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[tokio::test]
    // cargo psvm -v 1.4.0 -c
    // This version doesn't have the Plan.toml file, so it will fallback to Cargo.lock
    // and check if the versions in the local toml file comply with the Cargo.lock file.
    // This will fail because the Cargo.toml file has version 1.14.0 while we are checking
    // against 1.4.0
    async fn test_check_version_without_fallback_fails_with_incorrect_version() {
        let input_cargo_toml_path = Path::new("src/testing/plan-toml/check.Cargo.toml");
        let version = "1.4.0";

        let res = verify_version_checking(version, input_cargo_toml_path).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Dependencies are not up to date"
        );
    }

    #[tokio::test]
    // cargo psvm -v 1.3.0 -c
    // This version doesn't have the Plan.toml file, so it will fallback to Cargo.lock
    // and check if the versions in the local toml file comply with the Cargo.lock file
    async fn test_check_version_with_fallback_passes() {
        let input_cargo_toml_path = Path::new("src/testing/cargo-lock/output.Cargo.toml");
        let version = "1.3.0";

        let res = verify_version_checking(version, input_cargo_toml_path).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[tokio::test]
    // cargo psvm -v 1.5.0
    // This version has the Plan.toml file, so it will not fallback to Cargo.lock
    async fn test_dependency_branch_plan_update() {
        let input_toml_path = Path::new("src/testing/plan-toml/input.Cargo.toml");
        let expected_output_toml = include_str!("testing/plan-toml/output.Cargo.toml");
        let version = "1.5.0";

        verify_version_mapping(version, input_toml_path, expected_output_toml).await;
    }

    #[tokio::test]
    // cargo psvm -v 1.6.0 -O
    // This version is present in the ORML repository, so it will fetch the ORML crates and update
    // the Cargo.toml file with the new versions
    async fn test_orml_version_mapping_passes() {
        let input_cargo_toml_path = Path::new("src/testing/orml/input.Cargo.toml");
        let output_cargo_toml_path = include_str!("testing/orml/output.Cargo.toml");
        let version = "1.6.0";

        verify_orml_version_mapping(version, input_cargo_toml_path, output_cargo_toml_path).await;
    }

    #[tokio::test]
    // cargo psvm -v 1.14.0 -O
    // This version is not present in the ORML repository, so it will not fetch the ORML crates and update
    // the Cargo.toml file with only the polkadot-sdk versions
    async fn test_orml_mapping_without_branch_passes() {
        let input_cargo_toml_path = Path::new("src/testing/orml/input.Cargo.toml");
        let output_cargo_toml_path = include_str!("testing/orml/notOrml.Cargo.toml");
        let version = "1.14.0";

        verify_orml_version_mapping(version, input_cargo_toml_path, output_cargo_toml_path).await;
    }

    #[tokio::test]
    // cargo psvm -v 1.6.0
    // This version is present in the ORML repository, but the --orml flag isn't supplied so it will not touch
    // the ORML crates and update the Cargo.toml file with only the polkadot-sdk versions
    async fn test_orml_mapping_without_flag_passes() {
        let input_cargo_toml_path = Path::new("src/testing/orml/input.Cargo.toml");
        let output_cargo_toml_path = include_str!("testing/orml/noFlag.Cargo.toml");
        let version = "1.6.0";

        verify_version_mapping(version, input_cargo_toml_path, output_cargo_toml_path).await;
    }

    #[tokio::test]
    async fn test_parse_version_mapping_from_plan_toml() {
        let response = r#"
[[crate]]
name = "package_minor"
from = "0.1.0"
to = "0.1.1"
bump = "minor"
reason = "bumped by --patch"

[[crate]]
name = "package_major"
from = "1.0.0"
to = "2.0.0"
bump = "major"
reason = "changed"

[[crate]]
name = "package_no_publish"
from = "0.1.0"
to = "0.1.0"
bump = "major"
publish = false
"#;
        let version = "N.N.N";
        let source = "Plan.toml";

        let _m = mockito::mock(
            "GET",
            format!(
                "/paritytech/polkadot-sdk/release-crates-io-v{}/{}",
                version, source
            )
            .as_str(),
        )
        .with_status(200)
        .with_body(response)
        .create();

        let git_server = &mockito::server_url();
        let mapping = get_version_mapping_with_fallback(git_server, version)
            .await
            .unwrap();

        assert_eq!(mapping.len(), 2);
        assert_eq!(mapping.get("package_minor"), Some(&"0.1.1".to_string()));
        assert_eq!(mapping.get("package_major"), Some(&"2.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_parse_version_mapping_from_cargo_lock() {
        let response = r#"
[[package]]
name = "local_package"
version = "0.1.0"

[[package]]
name = "remote_package"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let version = "N.N.N";
        let source = "Cargo.lock";

        let _m = mockito::mock(
            "GET",
            format!(
                "/paritytech/polkadot-sdk/release-crates-io-v{}/{}",
                version, source
            )
            .as_str(),
        )
        .with_status(200)
        .with_body(response)
        .create();

        let git_server = &mockito::server_url();
        let mapping = get_version_mapping_with_fallback(git_server, version)
            .await
            .unwrap();

        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping.get("local_package"), Some(&"0.1.0".to_string()));
    }

    #[tokio::test]
    // This test will fetch all available versions, update a generic parachain Cargo.toml file
    // and assert that the Cargo.toml file has been updated (modified)
    // This is not exhaustive, but it's a good way to ensure that the logic works for all versions
    // To run this test, ensure you have installed the GitHub CLI and are authenticated
    // cause it will fetch the latest release branches from the GitHub API
    async fn works_for_all_versions() {
        let release_versions = crate::versions::get_polkadot_sdk_versions().await.unwrap();

        for version in release_versions {
            let crates_versions =
                get_version_mapping_with_fallback(crate::DEFAULT_GIT_SERVER, &version)
                    .await
                    .unwrap();

            assert!(
                crates_versions.len() > 0,
                "No versions found for {}",
                version
            );

            let input_cargo_toml_path = Path::new("src/testing/plan-toml/input.Cargo.toml");
            let result = crate::update_dependencies_impl(
                &input_cargo_toml_path,
                &crates_versions,
                false,
                false,
            )
            .unwrap();

            assert!(result.is_some()); // If no changes are made, the result will be None
        }
    }

    #[tokio::test]
    // This test will fetch all available versions, update a generic parachain Cargo.toml file
    // and assert that the Cargo.toml file has been updated (modified)
    // This is not exhaustive, but it's a good way to ensure that the logic works for all orml versions
    // To run this test, ensure you have installed the GitHub CLI and are authenticated
    // cause it will fetch the latest release branches from the GitHub API
    async fn works_for_all_orml_versions() {
        let release_versions = crate::versions::get_release_branches_versions(Repository::Orml)
            .await
            .unwrap();

        for version in release_versions {
            let mut crates_versions =
                get_version_mapping_with_fallback(crate::DEFAULT_GIT_SERVER, &version)
                    .await
                    .unwrap();

            let orml_crates_version =
                get_orml_crates_and_version(crate::DEFAULT_GIT_SERVER, &version)
                    .await
                    .unwrap();
            include_orml_crates_in_version_mapping(&mut crates_versions, orml_crates_version);

            assert!(
                crates_versions.len() > 0,
                "No versions found for {}",
                version
            );

            let input_cargo_toml_path = Path::new("src/testing/orml/input.Cargo.toml");
            let result = crate::update_dependencies_impl(
                &input_cargo_toml_path,
                &crates_versions,
                false,
                false,
            )
            .unwrap();

            assert!(result.is_some()); // If no changes are made, the result will be None
        }
    }
}
