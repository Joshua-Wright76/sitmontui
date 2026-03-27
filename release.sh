#!/bin/bash
set -e

# Release script for sitmon_cli (ARM64 only)
# Usage: ./release.sh [major|minor|patch]

cd "$(dirname "$0")"

# Get current version
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $CURRENT_VERSION"

# Parse version parts
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Increment version based on argument
case "${1:-patch}" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
    *)
        echo "Usage: $0 [major|minor|patch]"
        exit 1
        ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"
echo "New version: $NEW_VERSION"

# Build ARM64 release
echo "Building ARM64..."
cargo build --release

# Copy binary to release location
mkdir -p release
cp target/release/sitmon_cli release/sitmon_cli

# Get SHA256 checksum
SHA256=$(shasum -a 256 release/sitmon_cli | cut -d' ' -f1)
echo "ARM64 SHA256: $SHA256"

# Update version in Cargo.toml
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Commit version bump
git add Cargo.toml
git commit -m "Bump version to $NEW_VERSION"

# Create and push tag
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
echo "Pushing tag v$NEW_VERSION..."
git push origin "v$NEW_VERSION"

# Push commit
git push origin main

# Create GitHub release
echo "Creating GitHub release..."
gh release create "v$NEW_VERSION" \
    --title "Sitmon v$NEW_VERSION" \
    --notes "See CHANGELOG for details" \
    release/sitmon_cli

echo ""
echo "Release v$NEW_VERSION complete!"
echo "ARM64 SHA256: $SHA256"
echo "Don't forget to update the Homebrew tap formula with the new version and SHA256."
