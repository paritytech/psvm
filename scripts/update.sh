#!/bin/bash

set -eu

PDSKVM_DIR="$(git rev-parse --show-toplevel)"
PROCESS_SCRIPT="$PDSKVM_DIR/scripts/process.sh"
VERSIONS_DIR="$PDSKVM_DIR/src/versions/"

# Ensure get_version.sh is executable
chmod +x "$PROCESS_SCRIPT"

TARGET_DIR="$1"

# Validate the target directory
if [[ $TARGET_DIR != */polkadot-sdk ]]; then
  TARGET_DIR="../polkadot-sdk"
fi

# Hardcoded list of branches to checkout
BRANCHES=("release-crates-io-v1.3.0" "release-crates-io-v1.4.0" "release-crates-io-v1.5.0" "release-crates-io-v1.6.0" "release-crates-io-v1.7.0" "release-crates-io-v1.8.0" "release-crates-io-v1.9.0")

# Navigate to the target directory
cd "$TARGET_DIR" || exit

# Loop through each branch, checkout, execute get_version.sh, and move the output
for BRANCH in "${BRANCHES[@]}"; do
  git stash -u
  git checkout $BRANCH

  echo "Processing $BRANCH"
  
  # Execute get_version.sh and redirect output to a json file named after the branch
  $PROCESS_SCRIPT
  
  # Move the output json to the src/versions directory
  mv "${BRANCH}.json" "$VERSIONS_DIR"
done