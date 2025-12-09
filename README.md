# git-forge

A simple CLI tool for basic interactions with issues and pull requests across GitHub, GitLab, Gitea, and Forgejo.

## Usage

`git-forge` requires `Node.js` to run. Other than node, `git-forge` only uses dev dependencies.

```sh
git forge [<subcommand>] [<options>]
```

### Subcommands

- `issue` - List issues
- `pr` - List pull requests and create a pull request for the current branch
- `web` - Get the web URL for repositories

Note that due to differing forge APIs, some behavior may vary across forges. Use `--help` for detailed information.

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

```sh
npm install
npm run build
```

Move `dist/git-forge` to a PATH directory.

## Support Me :heart:

If you like this project, you can support me with [GitHub Sponsors](https://github.com/sponsors/leleat).

## License

MIT. See the license file for details.
