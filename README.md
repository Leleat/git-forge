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
- `browse` - Open repository links in your browser or print them to stdout (repository home, issues, PRs, commits, files)
- `completions` - generate shell completions. See [limitations](#shell-completions) below.

Note that due to differing forge APIs, some behavior may vary across forges. E.g. filtering of PRs may happen client-side for some forges, while it happens server-side for others.

### Example Use Cases

```sh
# git aliases in .gitconfig
[alias]
    # Search for and copy an issue link to clipboard. `copy` is a custom script
    fcpissue = "!git forge issue list | fzf | cut -f 3 | copy"
    # Search for a PR and check it out locally
    freviewpr = "!git forge pr list | fzf | cut -f 1 | xargs git forge pr checkout"
```

### Shell Completions

You can generate shell completions for `bash`, `zsh`, `fish`, `powershell`, and `elvish` using:

```sh
git forge completions <shell>
```

You can save the completion script somewhere and source it in your shell configuration.

**But** there is a limitation. The generated completions work for `git-forge` but may not work for `git forge` (note the lack of the hyphen between `git` and `forge`)  without modification.

`git forge` uses git's completion. Here you can read about [git's completion setup](https://git-scm.com/book/en/v2/Appendix-A%3A-Git-in-Other-Environments-Git-in-Bash). The generated git-forge completion doesn't integrate well with git's completion since it relies on certain arguments and variables. So you may need to do some manual wiring.

Let's look at bash as an example: The git completion script looks for the function `_git_<command>` when `git <command>` is entered, e.g. `_git_forge()` when typing `git forge`. The `git-forge` completion script defines a function based on the binary name: `_git-forge` (note the hypen instead of the underscore), which adds tab-completion for `git-forge`. You could add a `_git_forge` function that delegates to `_git-forge` to get the completions for `git forge` working:

(Other shells may have similar issues - feedback welcome)

```bash
# This is your generated bash completion script, e.g. git-forge.bash when
# running `git forge completions bash > git-forge.bash`. It already contains
# _git-forge() etc...

# Now *append* the following function
_git_forge() {
    # Adjust COMP_WORDS and COMP_CWORD to make it look like we're completing
    # 'git-forge' instead of 'git forge'
    local -a adjusted_words=("git-forge" "${COMP_WORDS[@]:2}")
    local adjusted_cword=$((COMP_CWORD - 1))

    # Temporarily override COMP_WORDS and COMP_CWORD
    local -a save_words=("${COMP_WORDS[@]}")
    local save_cword=$COMP_CWORD
    COMP_WORDS=("${adjusted_words[@]}")
    COMP_CWORD=$adjusted_cword

    # Call the original completion function with adjusted arguments
    _git-forge "git-forge" "${COMP_WORDS[COMP_CWORD]}" "${COMP_WORDS[COMP_CWORD-1]}"

    # Restore original values
    COMP_WORDS=("${save_words[@]}")
    COMP_CWORD=$save_cword
}
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
