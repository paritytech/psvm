# Polkadot SDK Version Manager

This is a simple tool to manage and update the Polkadot SDK dependencies in any Cargo.toml file. It [works offline](#offline) using a local version mapping previously fetched from the Polkadot SDK crates-io branches, and [works online](#online) by getting directly the versions from the Polkadot SDK crates-io branches (`Plan.toml` or `Cargo.lock`) files.

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

```sh
# Go to the directory containing the Cargo.toml file you want to update
cd <cargo-toml-dir>
# Run the psvm command to update the dependencies using PSVM local versions
psvm
# You can also update it using the path to the Cargo.toml file
psvm -p <cargo-toml-dir>/Cargo.toml
# Overwrite local dependencies with crates.io versions
psvm -o
# Update to a specific Polkadot SDK version (default 1.3.0) using PSVM local versions
psvm -v "1.7.0"
# Update to the versions defined in a specific Polkadot SDK branch Plan.toml file
# i.e https://raw.githubusercontent.com/paritytech/polkadot-sdk/release-crates-io-v1.6.0/Plan.toml
psvm -b "release-crates-io-v1.6.0"
# Update to the versions defined in a specific Polkadot SDK branch Cargo.lock file
# i.e https://raw.githubusercontent.com/paritytech/polkadot-sdk/release-crates-io-v1.3.0/Cargo.lock
psvm -b "release-crates-io-v1.3.0" -s "Cargo.lock"
```

### Offline

You can use PSVM offline with the `-v` or `--version` flag, cause PSVM keeps a local mapping (i.e. [v1.3.0 Mapping](/src/versions/release-crates-io-v1.3.0.json)) of the Polkadot SDK dependencies versions, that will be slowly updated. So be aware that the latest Polkadot SDK versions may not be available in the local mapping.

### Online

To have the latest versions, you can use the `-b` or `--branch` flag to fetch the versions from a specific Polkadot SDK branch. This will fetch the `Plan.toml` file (used to publish crates into crates.io) from the branch and update the local mapping with the versions defined in the file (i.e [v1.6.0 `Plan.toml`](https://raw.githubusercontent.com/paritytech/polkadot-sdk/release-crates-io-v1.6.0/Plan.toml)). However, this could not work for older branches (i.e. v1.3.0), cause the `Plan.toml` file may not exist. In this case, you can use the `-s` or `--source` flag to fetch the `Cargo.lock` file (i.e. [v1.3.0 `Cargo.lock`](https://raw.githubusercontent.com/paritytech/polkadot-sdk/release-crates-io-v1.3.0/Cargo.lock)) from the branch and update the local mapping with the versions defined in the file.

## Maintenance  

To update the Polkadot SDK dependencies mapping, run `./scripts/update.sh <polkadot-sdk repo path>`. This will update the dependencies `json` files located in `src/versions` with the latest versions defined in Polkadot SDK crates-io branches.

You can add more branches in the `BRANCHES` list from `./scripts/update.sh` file. Once you have process the branches, you need to index them in the `src/versions.rs` file.
