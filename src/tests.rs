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
    use crate::versions::get_version_mapping_with_fallback;
    use std::path::Path;

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
            crate::update_dependencies_impl(&input_cargo_toml_path, &crates_versions, false)
                .unwrap();
            
        // Assert that the result matches the expected output
        assert_eq!(result, Some(expected_cargo_toml.into()));
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
    // cargo psvm -v 1.5.0
    // This version has the Plan.toml file, so it will not fallback to Cargo.lock
    async fn test_dependency_branch_plan_update() {
        let input_toml_path = Path::new("src/testing/plan-toml/input.Cargo.toml");
        let expected_output_toml = include_str!("testing/plan-toml/output.Cargo.toml");
        let version = "1.5.0";

        verify_version_mapping(version, input_toml_path, expected_output_toml).await;
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
        let release_versions = crate::versions::get_release_branches_versions()
            .await
            .unwrap();

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
            let result =
                crate::update_dependencies_impl(&input_cargo_toml_path, &crates_versions, false)
                    .unwrap();

            assert!(result.is_some()); // If no changes are made, the result will be None
        }
    }
}
