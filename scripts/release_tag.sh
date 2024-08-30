#!/bin/bash

if [ -z $1 ]; then
    echo "Usage: $0 <new_version>"
    exit 1
fi

NEW_VER=$1
echo "Updating Tari Payment Server to $NEW_VER"
# List of Cargo.toml files to update
CARGO_FILES=($(find . -name 'Cargo.toml'))
for f in ${CARGO_FILES[@]}; do
    # Update the version number
    sed -i "s/^version = \".*\"/version = \"$1\"/" $f
    for f2 in "${CARGO_FILES[@]}"; do
        # If f2 is the same as f, skip
        if [ "$f" == "$f2" ]; then
            continue
        fi
        # Extract the package name from the file path
        PACKAGE=$(basename "$(dirname "$f2")")
        # if package is the current directory (.), skip
        if [ "$PACKAGE" == "." ]; then
            continue
        fi

        #echo "Setting ${PACKAGE} in $f to $NEW_VER.."
        # Use sed to update the version number for the specific package
        sed -i "s/${PACKAGE}\s*=\s*{\s*version\s*=\s*\"[0-9\.]*\"/${PACKAGE} = { version = \"$NEW_VER\"/" "$f"
    done
    echo "Updated $f"
done
git checkout -b "release-${NEW_VER}"
git commit -am "Updated version to $NEW_VER"
echo "Tagging new release"
git tag v${NEW_VER}
git push origin "release-v${NEW_VER}"
git push origin v${NEW_VER}
echo "Release tag updated"

