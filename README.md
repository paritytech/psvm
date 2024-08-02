# Polkadot SDK Version Manager

This is a simple tool to manage and update the Polkadot SDK dependencies in any Cargo.toml file. It will automatically update the Polkadot SDK dependencies to their correct crates.io version.

## Installation

From [GitHub](https://github.com/paritytech/psvm):

```sh
cargo install --git https://github.com/paritytech/psvm psvm
```

From [crates.io](https://crates.io/crates/psvm):

```sh
cargo install psvm
```

## Usage

Go to the directory containing the Cargo.toml file you want to update and run `psvm`. This will automatically update the Polkadot SDK dependencies in the Cargo.toml file to their correct crates.io version.

If you want to update the local dependencies (using `path="..."`), you can use the `-o` or `-overwrite` flag, this will remove the `path` and set a crates.io version instead.

If you want to update the dependencies to a specific Polkadot SDK version, you can use the `-v` or `--version` flag, followed by the version you want to update to.

If you want to check if the dependencies in your local Cargo.toml file are matching to a specific Polkadot SDK version, you can use the `-c` or `--check` flag along with the `--version` flag followed by the version you want to check against.

```sh
# Go to the directory containing the Cargo.toml file you want to update
cd <cargo-toml-dir>
# Update to a specific Polkadot SDK version
psvm -v "1.3.0"
# You can also update an specific Cargo.toml file by passing its path
psvm -v "1.4.0" -p <cargo-toml-dir>/Cargo.toml
# Overwrite local dependencies (with same name as Polkadot SDK crates) with crates.io versions
psvm -v "1.7.0" -o
# List all available Polkadot SDK versions
psvm -l
# Check against a particular Polkadot SDK version without updating the Cargo.toml file
psmv -v "1.4.0" -c
```

> Listing all available Polkadot SDK versions requires querying the GitHub API, so your IP may be rate-limited. If a rate limit is reached, the tool will fallback to the GitHub CLI to list the versions. Ensure you have the GitHub CLI installed and authenticated to avoid any issue.

## Workflow

To update a `Cargo.toml`, the tool will fetch the `Plan.toml` file (used to publish crates into crates.io) from the release branch in Polkadot SDK associated to the version input (`--version` argument), generate a mapping (crate -> version) filtering all crates that were not published in this released (i.e. `publish = false`) **but keeping the [crates published by `parity-crate_owner`](https://crates.io/users/parity-crate-owner) (even if they were not published in this release)**, and overwrite the input Cargo.toml file to match the version from the mapping (i.e [v1.6.0 `Plan.toml`](https://raw.githubusercontent.com/paritytech/polkadot-sdk/release-crates-io-v1.6.0/Plan.toml)).

In specific versions, the `Plan.toml` file may not exists (i.e. v1.3.0). In this case, the tool will fallback to the `Cargo.lock` file (i.e. [v1.3.0 `Cargo.lock`](https://raw.githubusercontent.com/paritytech/polkadot-sdk/release-crates-io-v1.3.0/Cargo.lock)) from the branch, generate a mapping using this file and overwrite the input Cargo.toml file to match the version from the mapping. The only concern to be aware in this scenario is that the `Cargo.lock` file may contain dependencies that are not published in crates.io, and the tool will not be able to filter them out cause it is not possible to determine if a crate is published or not (with this file). If you have a local dependency with a name similar to a crate not published, the tool will overwrite it, so be careful. Currently, this only happens with v1.3.0, but as the branches can change at any time, it is important to be aware of this. The tool will alert with a message "Failed to get Plan.toml, falling back to Cargo.lock." if this happens.
