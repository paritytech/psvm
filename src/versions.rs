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
