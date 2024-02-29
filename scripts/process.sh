#!/bin/bash

# This script generates a JSON file containing the name and version of all crates 
# in the current directory and subdirectories that do not contain publish = false in their Cargo.toml file.
# The JSON file is named after the current Git branch and has a .json extension.
# Run this in Polkadot SDK branches release-crates-io-vX.Y.Z to generate the JSON file.
# Copy the JSON file in src/versions and index it in src/versions.rs

# Get the current Git branch name
branch_name=$(git rev-parse --abbrev-ref HEAD)

# Output file named after the current branch with a .json extension
json_output_file="${branch_name}.json"

# Begin the JSON file with an opening brace
echo "{" > "$json_output_file"

# Keep track of the first entry to handle commas correctly
first_entry=true

# Find all Cargo.toml files in the current directory and subdirectories
find . -not \( -path './target*' -prune \) -not \( -path './.git*' -prune \) -name Cargo.toml | while read cargo_file; do
    # Check if the Cargo.toml file does not contain publish = false
    if ! grep -q 'publish\s*=\s*false' "$cargo_file"; then
        # Extract the package name and version from each Cargo.toml file
        name=$(grep '^name = ' "$cargo_file" | head -n 1 | cut -d '"' -f 2)
        version=$(grep '^version = ' "$cargo_file" | head -n 1 | cut -d '"' -f 2)

        # Check if both name and version are found
        if [ ! -z "$name" ] && [ ! -z "$version" ] && [[ ! "$name" =~ -polkadot-runtime$ ]] && [[ ! "$name" =~ -kusama-runtime$ ]]; then
            # Append a comma before the next entry if it's not the first
            if [ "$first_entry" = true ]; then
                first_entry=false
            else
                echo "," >> "$json_output_file"
            fi

            # Append the crate name and version to the JSON output file
            echo -n "   \"$name\": \"$version\"" >> "$json_output_file"
        fi
    fi
done

# End the JSON file with a closing brace
echo "
}" >> "$json_output_file"