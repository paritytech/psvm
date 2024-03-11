# Polkadot SDK Version Manager

This is a simple tool to manage and update the Polkadot SDK dependencies in any Cargo.toml file.

## Installation

```sh
cargo install --git https://github.com/paritytech/psvm psvm
```

## Usage

Go to the directory containing the Cargo.toml file you want to update and run `psvm`. This will automatically update the Polkadot SDK dependencies in the Cargo.toml file to their correct crates.io version.

If you want to update the local dependencies (using `path="..."`), you can use the `-o` or `-overwrite` flag, this will remove the `path` and set a crates.io version instead.

If you want to update the dependencies to a specific Polkadot SDK version, you can use the `-v` or `--version` flag, followed by the version you want to update to.

```sh
# Go to the directory containing the Cargo.toml file you want to update
cd <cargo-toml-dir>
# Run the psvm command to update the dependencies
psvm
# You can also update it using the path to the Cargo.toml file
psvm -p <cargo-toml-dir>/Cargo.toml
# Overwrite local dependencies with crates.io versions
psvm -o
# Update to a specific Polkadot SDK version (default 1.3.0)
psvm -v "1.7.0"
```

## Maintenance  

To update the Polkadot SDK dependencies mapping, run `./scripts/update.sh <polkadot-sdk repo path>`. This will update the dependencies `json` files located in `src/versions` with the latest versions defined in Polkadot SDK crates-io branches.

You can add more branches in the `BRANCHES` list from `./scripts/update.sh` file. Once you have process the branches, you need to index them in the `src/versions.rs` file.
