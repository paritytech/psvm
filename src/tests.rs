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
    use std::collections::BTreeMap;
    use std::path::Path;

    #[test]
    // cargo psvm --version 1.3.0
    fn test_dependency_version_update() {
        // Example input TOML and expected output TOML as strings
        let input_toml_path = Path::new("src/testing/cargo-lock/input.Cargo.toml");
        let expected_output_toml = include_str!("testing/cargo-lock/output.Cargo.toml");

        // Example versions data as a string
        let crates_versions_data = include_str!("versions/release-crates-io-v1.3.0.json");
        let crates_versions: BTreeMap<String, String> =
            serde_json::from_str(crates_versions_data).unwrap();

        // Call the refactored logic function with the test data
        let result =
            crate::update_dependencies_impl(&input_toml_path, &crates_versions, false).unwrap();

        // Assert that the result matches the expected output
        assert_eq!(result, Some(expected_output_toml.into()));
    }

    #[tokio::test]
    // cargo psvm --branch release-crates-io-v1.3.0
    async fn test_dependency_branch_cargo_update() {
        // Example input TOML and expected output TOML as strings
        let input_toml_path = Path::new("src/testing/cargo-lock/input.Cargo.toml");
        let expected_output_toml = include_str!("testing/cargo-lock/output.Cargo.toml");

        let branch = "release-crates-io-v1.3.0";
        let source = "Cargo.lock";

        let crates_versions =
            crate::versions::get_branch_mapping(crate::DEFAULT_GIT_SERVER, branch, source)
                .await
                .unwrap();

        // Call the refactored logic function with the test data
        let result =
            crate::update_dependencies_impl(&input_toml_path, &crates_versions, false).unwrap();

        // Assert that the result matches the expected output
        assert_eq!(result, Some(expected_output_toml.into()));
    }

    #[tokio::test]
    // cargo psvm --branch release-crates-io-v1.5.0 --source Plan.toml
    async fn test_dependency_branch_plan_update() {
        // Example input TOML and expected output TOML as strings
        let input_toml_path = Path::new("src/testing/plan-toml/input.Cargo.toml");
        let expected_output_toml = include_str!("testing/plan-toml/output.Cargo.toml");

        let branch = "release-crates-io-v1.5.0";
        let source = "Plan.toml";

        let crates_versions =
            crate::versions::get_branch_mapping(crate::DEFAULT_GIT_SERVER, branch, source)
                .await
                .unwrap();

        // Call the refactored logic function with the test data
        let result =
            crate::update_dependencies_impl(&input_toml_path, &crates_versions, false).unwrap();

        // Assert that the result matches the expected output
        assert_eq!(result, Some(expected_output_toml.into()));
    }

    #[tokio::test]
    async fn test_dependency_branch_cargo_mocked_update() {
        let response = r#"
[[package]]
name = "local_package"
version = "0.1.0"

[[package]]
name = "remote_package"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let branch = "release-crates-io-vN.N.N";
        let source = "Cargo.lock";

        let _m = mockito::mock(
            "GET",
            format!("/paritytech/polkadot-sdk/{}/{}", branch, source).as_str(),
        )
        .with_status(200)
        .with_body(response)
        .create();

        let git_server = &mockito::server_url();
        let mapping = crate::versions::get_branch_mapping(git_server, branch, source)
            .await
            .unwrap();

        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping.get("local_package"), Some(&"0.1.0".to_string()));
    }

    #[tokio::test]
    async fn test_dependency_branch_plan_mocked_update() {
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
        let branch = "release-crates-io-vN.N.N";
        let source = "Plan.toml";

        let _m = mockito::mock(
            "GET",
            format!("/paritytech/polkadot-sdk/{}/{}", branch, source).as_str(),
        )
        .with_status(200)
        .with_body(response)
        .create();

        let git_server = &mockito::server_url();
        let mapping = crate::versions::get_branch_mapping(git_server, branch, source)
            .await
            .unwrap();

        assert_eq!(mapping.len(), 2);
        assert_eq!(mapping.get("package_minor"), Some(&"0.1.1".to_string()));
        assert_eq!(mapping.get("package_major"), Some(&"2.0.0".to_string()));
    }
}
