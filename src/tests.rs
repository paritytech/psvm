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
    use serde_json::from_str;

    #[test]
    fn test_dependency_update() {
        // Example input TOML and expected output TOML as strings
        let input_toml_path = "src/testing/input.Cargo.toml";
        let expected_output_toml = include_str!("testing/output.Cargo.toml");

        // Example versions data as a string
        let crates_versions_data = include_str!("versions/release-crates-io-v1.3.0.json");
        let crates_versions: BTreeMap<String, String> = from_str(crates_versions_data).unwrap();

        // Call the refactored logic function with the test data
        let result = crate::update_dependencies_impl(input_toml_path, &crates_versions, false).unwrap();

        println!("{}", result);

        // Assert that the result matches the expected output
        assert_eq!(result, expected_output_toml);
    }
}
