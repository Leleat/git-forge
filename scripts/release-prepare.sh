#!/usr/bin/env bash

# release-prepare.sh
#
# See also ./release-create.sh
#
# Prepares a new release by:
# - Analyzing git history since the last release tag (v*)
# - Extracting certain conventional commits (feat, fix, breaking changes)
# - Calculating the new version using semantic versioning rules
# - Updating Cargo.toml with the new version and adding changes to CHANGELOG.md
#
# Requirements:
# - Must be run from the repository root
# - Commits must follow Conventional Commits format:
#   - feat: for new features
#   - fix: for bug fixes
#   - type!: or type(scope)!: for breaking changes
#
# Usage:
#   ./scripts/release-prepare.sh

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

get_initial_commit() {
    git rev-list --max-parents=0 HEAD
}

get_last_release_tag() {
    git tag -l 'v*' --sort=-version:refname | head -1 2>/dev/null
}

get_commit_range_for_changes() {
    local last_tag
    local initial_commit

    if last_tag=$(get_last_release_tag) && [ -n "$last_tag" ]; then
        echo "${last_tag}..HEAD"
    else
        initial_commit=$(get_initial_commit)
        echo "${initial_commit}..HEAD"
    fi
}

get_last_release_or_initial_commit() {
    local last_tag

    if last_tag=$(get_last_release_tag) && [ -n "$last_tag" ]; then
        echo "$last_tag"
    else
        get_initial_commit
    fi
}

parse_commits() {
    local range=$1
    local commits

    commits=$(git log "$range" --format="%s%x09%an" --no-merges 2>/dev/null || echo "")

    if [ -z "$commits" ]; then
        return
    fi

    # Extract breaking changes (has ! after type or scope)
    # Pattern: type! or type(scope)!
    BREAKING_CHANGES=$(echo "$commits" | grep -E "^(feat|fix|chore|docs|refactor|test|ci|perf|style|build|revert)(\([^)]*\))?!:" || true)

    # Extract features (feat: or feat(scope):, but not feat!:)
    FEATURES=$(echo "$commits" | grep -E "^feat(\([^)]*\))?:" | grep -vE "^feat(\([^)]*\))?!:" || true)

    # Extract fixes (fix: or fix(scope):, but not fix!:)
    FIXES=$(echo "$commits" | grep -E "^fix(\([^)]*\))?:" | grep -vE "^fix(\([^)]*\))?!:" || true)
}

strip_commit_message_prefix() {
    local commit_message=$1
    echo "${commit_message#*: }"
}

calculate_version() {
    local current_version
    local major minor patch

    current_version=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')

    if [ -z "$current_version" ]; then
        log_error "Could not parse version from $CARGO_TOML"
        exit 1
    fi

    IFS='.' read -r major minor patch <<< "$current_version"

    log_info "Current version: $current_version"

    if [ -n "$BREAKING_CHANGES" ]; then
        log_info "Breaking changes detected"

        # For 0.x.y versions, breaking changes bump minor
        if [ "$major" -eq 0 ]; then
            minor=$((minor + 1))
            patch=0

            log_info "Pre-1.0 version: bumping minor"
        else
            major=$((major + 1))
            minor=0
            patch=0

            log_info "Bumping major version"
        fi
    elif [ -n "$FEATURES" ]; then
        log_info "New features detected"

        minor=$((minor + 1))
        patch=0
    elif [ -n "$FIXES" ]; then
        log_info "Bug fixes detected"

        patch=$((patch + 1))
    else
        log_warn "No conventional commits for release found (feat, fix, or breaking changes)"

        exit 1
    fi

    NEW_VERSION="${major}.${minor}.${patch}"

    log_info "New version: $NEW_VERSION"
}

update_cargo_toml() {
    log_info "Updating $CARGO_TOML..."

    sed "/^\[package\]/,/^\[/{s/^version = \".*\"/version = \"$NEW_VERSION\"/;}" "$CARGO_TOML" > "${CARGO_TOML}.tmp"
    mv "${CARGO_TOML}.tmp" "$CARGO_TOML"

    log_info "Updated $CARGO_TOML to version $NEW_VERSION"
}

build_commit_list_for_changelog_entry() {
    local commits=$1
    local result=""

    while IFS= read -r commit; do
        if [ -n "$commit" ]; then
            local message="${commit%$'\t'*}"
            local author="${commit##*$'\t'}"

            message=$(strip_commit_message_prefix "$message")

            if [ "$author" != "Leleat" ]; then
                message="${message} by **${author}**"
            fi

            result="${result}- ${message}\n"
        fi
    done <<< "$commits"

    echo -e "$result"
}

build_changelog_entry() {
    local date_str
    date_str=$(date +%Y-%m-%d)

    local changelog_entry="## ${NEW_VERSION} (${date_str})\n\n"

    if [ -n "$BREAKING_CHANGES" ]; then
        changelog_entry="${changelog_entry}### Breaking Changes\n\n"
        changelog_entry="${changelog_entry}$(build_commit_list_for_changelog_entry "$BREAKING_CHANGES")\n\n"
    fi

    if [ -n "$FEATURES" ]; then
        changelog_entry="${changelog_entry}### Features\n\n"
        changelog_entry="${changelog_entry}$(build_commit_list_for_changelog_entry "$FEATURES")\n\n"
    fi

    if [ -n "$FIXES" ]; then
        changelog_entry="${changelog_entry}### Bugfixes\n\n"
        changelog_entry="${changelog_entry}$(build_commit_list_for_changelog_entry "$FIXES")\n\n"
    fi

    local old_ref
    old_ref=$(get_last_release_or_initial_commit)

    changelog_entry="${changelog_entry}---\n\n"
    changelog_entry="${changelog_entry}Full Changelog: [${old_ref}...v${NEW_VERSION}](${REPO_URL}/compare/${old_ref}...v${NEW_VERSION})"

    echo -e "$changelog_entry"
}

update_changelog() {
    local changelog_entry
    local temp_file="${CHANGELOG}.tmp"

    log_info "Updating $CHANGELOG..."

    changelog_entry=$(build_changelog_entry)

    # Insert changelog_entry after "# Changelog" header
    # Add blank line, then changelog_entry, then blank line, then rest of file
    {
        echo "# Changelog"
        echo ""
        echo -e "$changelog_entry"
        echo ""
        tail -n +2 "$CHANGELOG"
    } > "$temp_file"

    mv "$temp_file" "$CHANGELOG"

    log_info "Updated $CHANGELOG"
}

print_summary() {
    log_info ""
    log_info "═══════════════════════════════════════════"
    log_info "Release preparation complete!"
    log_info "═══════════════════════════════════════════"
    log_info "New Version: $NEW_VERSION"
    echo ""
    echo "Next steps:"
    echo "  1. Review and commit changes"
    echo "  2. Create a PR and merge changes"
    echo "  3. Pull changes"
    echo "  4. Run ./scripts/release-create.sh"
}

main() {
    assert_cwd_is_repo_root

    log_info "Preparing release..."

    COMMIT_RANGE=$(get_commit_range_for_changes)

    log_info "Analyzing commits in range: $COMMIT_RANGE"

    if [ -z "$(git log "$COMMIT_RANGE" --oneline --no-merges 2>/dev/null || echo "")" ]; then
        log_info "No new commits since last release"
        exit 1
    fi

    parse_commits "$COMMIT_RANGE"
    calculate_version
    update_cargo_toml
    update_changelog
    print_summary
}

CARGO_TOML="Cargo.toml"
CHANGELOG="CHANGELOG.md"
REPO_URL=$(grep '^repository = ' "$CARGO_TOML" | sed 's/repository = "\(.*\)"/\1/')

BREAKING_CHANGES=""
FEATURES=""
FIXES=""
NEW_VERSION=""
COMMIT_RANGE=""

main "$@"
