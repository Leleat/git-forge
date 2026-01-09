#!/usr/bin/env bash

# release-create.sh
#
# See also ./release-prepare.sh
#
# Creates a new release by:
# - Pulling changes from main
# - Verifying the last commit updated CHANGELOG.md and Cargo.toml
# - Extracting version from Cargo.toml
# - Creating an annotated tag (release notes extracted by GitHub Actions from CHANGELOG.md)
# - Pushing the tag to trigger GitHub Actions release workflow
#
# Requirements:
# - Must be run from the repository root
# - Must be on main branch
# - Last commit must have updated both CHANGELOG.md and Cargo.toml
#
# Usage:
#   ./scripts/release-create.sh

set -euo pipefail

log_info() { echo "[INFO] $1"; }
log_warn() { echo "[WARN] $1"; }
log_error() { echo "[ERROR] $1"; }

assert_cwd_is_repo_root() {
    if [ ! -d ".git" ]; then
        log_error "Must run from repository root (.git not found)"
        exit 1
    fi
}

assert_on_main_branch() {
    local current_branch
    current_branch=$(git rev-parse --abbrev-ref HEAD)

    if [ "$current_branch" != "main" ]; then
        log_error "Must be on main branch (currently on $current_branch)"
        exit 1
    fi
}

pull_latest_changes() {
    log_info "Pulling latest changes from main..."
    git pull origin main
}

assert_last_commit_updated_release_files() {
    local changed_files
    changed_files=$(git diff --name-only HEAD~1 HEAD)

    if ! echo "$changed_files" | grep -q "^CHANGELOG.md$"; then
        log_error "Last commit did not update CHANGELOG.md"
        log_error "Did you run ./scripts/release-prepare.sh and commit the changes?"
        exit 1
    fi

    if ! echo "$changed_files" | grep -q "^Cargo.toml$"; then
        log_error "Last commit did not update Cargo.toml"
        log_error "Did you run ./scripts/release-prepare.sh and commit the changes?"
        exit 1
    fi
}

get_version_from_cargo_toml() {
    local version
    version=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')

    if [ -z "$version" ]; then
        log_error "Could not parse version from $CARGO_TOML"
        exit 1
    fi

    echo "$version"
}

create_tag_message() {
    local version=$1
    echo "Release v${version}

See CHANGELOG.md for release notes."
}

create_annotated_tag() {
    local version=$1
    local tag_message=$2
    local tag_name="v${version}"

    log_info "Creating annotated tag $tag_name..."

    git tag -a "$tag_name" -m "$tag_message"

    log_info "Created tag $tag_name"
}

push_tag() {
    local version=$1
    local tag_name="v${version}"

    log_info "Pushing tag $tag_name to origin..."

    git push origin "$tag_name"

    log_info "Pushed tag $tag_name"
}

print_summary() {
    local version=$1
    local tag_name="v${version}"

    log_info ""
    log_info "═══════════════════════════════════════════"
    log_info "Release created successfully!"
    log_info "═══════════════════════════════════════════"
    log_info "Tag: $tag_name"
    log_info ""
    log_info "A GitHub Action will now create a draft release"
    log_info "Monitor progress at: ${REPO_URL}/actions"
    log_info "Review and publish the draft release at: ${REPO_URL}/releases"
}

main() {
    assert_cwd_is_repo_root
    assert_on_main_branch

    log_info "Creating release..."

    pull_latest_changes
    assert_last_commit_updated_release_files

    log_info "Version: $VERSION"

    TAG_MESSAGE=$(create_tag_message "$VERSION")

    create_annotated_tag "$VERSION" "$TAG_MESSAGE"
    push_tag "$VERSION"
    print_summary "$VERSION"
}

CARGO_TOML="Cargo.toml"
REPO_URL=$(grep '^repository = ' "$CARGO_TOML" | sed 's/repository = "\(.*\)"/\1/')
VERSION=$(get_version_from_cargo_toml)

main "$@"
