# git-forge

A simple CLI tool for basic interactions with issues and pull requests across GitHub, GitLab, Gitea, and Forgejo.

> [!WARNING]
> This project is mostly for me to practice the Rust programming language.

## Usage

```sh
git forge [<subcommand>] [<options>]
```

### Subcommands

- `issue` - List issues
- `pr` - List pull requests, create a pull request, and checkout a pull request
- `web` - Get the web URLs for the repository, for the issues, and for the PRs

Note that due to differing forge APIs, some behavior may vary across forges. E.g. filtering of PRs may happen client-side for some forges, while it happens server-side for others.

### Example Use Cases

```sh
# git aliases in .gitconfig
[alias]
    # Search for and copy an issue link to clipboard. `copy` is a custom script
    fcpissue = "!git forge issue | fzf | cut -f 2 | copy"
    # Search for and open an issue in your browser (on linux)
    fopenissue = "!git forge issue | fzf | cut -f 2 | xargs xdg-open &> /dev/null"
    # Open the issues page on a git forge (on linux); e.g. https://github.com/Leleat/git-forge/issues
    fopenissues = "!git forge web --type issues | xargs xdg-open &> /dev/null"
    # Search for a PR and check it out locally
    freviewpr = "!git forge pr | fzf | cut -d' ' -f 1 | xargs git forge pr checkout"
```

## Installation

Clone the repository. Then run

```sh
# First, cd into the <GIT_REPO>
cargo build --release
```

Move `target/release/git-forge` to a `$PATH` directory.

## Support Me

If you like this project, you can support me with [GitHub Sponsors](https://github.com/sponsors/leleat).

## License

MIT. See the license file for details.
