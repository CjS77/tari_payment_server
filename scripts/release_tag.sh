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
        echo "Updating dependencies in $f2"
        # Extract the package name from the file path
        PACKAGE=$(basename "$(dirname "$f2")")
        # if package is the current directory (.), skip
        if [ "$PACKAGE" == "." ]; then
            continue
        fi
        echo "    Updating ${PACKAGE} to $NEW_VER"


        #echo "Setting ${PACKAGE} in $f to $NEW_VER.."
        # Use sed to update the version number for the specific package
        sed -i "s/${PACKAGE}\s*=\s*{\s*version\s*=\s*\"[^\"]*\"/${PACKAGE} = { version = \"$NEW_VER\"/" "$f"
    done
    echo "Updated $f"
done
REL_VER="v${NEW_VER}"
git checkout -b "release-${REL_VER}"
cargo update
git commit -am "Updated version to $REL_VER"
echo "Tagging new release: $REL_VER"
git tag ${REL_VER}
git push origin "release-${REL_VER}"
git push origin ${REL_VER}
echo "Release tag updated"

