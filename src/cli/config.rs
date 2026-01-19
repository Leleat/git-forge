//! The `config` subcommand.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs,
};

use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use dialoguer::Editor;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{forge::ApiType, issue::IssueState, pr::PrState},
    git::{self, GitRemoteData},
    io::OutputFormat,
};

const APP_NAME: &str = std::env!("CARGO_PKG_NAME");
const CONFIG_NAME: &str = "config";
const DEFAULT_REMOTE: &str = "origin";
const DEFAULT_SET_CMD_SCOPE: &str = "global";

// =============================================================================
// CLI Arguments
// =============================================================================

/// Command-line arguments for the `config` subcommand.
#[derive(Args)]
pub struct ConfigCommandArgs {
    #[command(subcommand)]
    pub subcommand: ConfigCommand,
}

/// Available subcommands for config subcommand.
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Get configuration value(s).
    Get(ConfigGetArgs),

    /// Set a configuration value.
    Set(ConfigSetArgs),

    /// Unset a configuration value.
    #[command(alias = "delete")]
    Unset(ConfigUnsetArgs),

    /// Edit the configuration file.
    Edit,
}

const PATH_DEFINITION: &str = "A path follows the format [<COMMAND_PATH>/]<FLAG>, e.g. editor, pr/editor, or pr/create/editor.
The value with a more specific path within the precedence hierarchy of a single scope wins. For instance, when executing \"git-forge pr create\", we look for the config value with the following paths: first pr/create/editor, then pr/editor, and finally editor in the remote scope. If no value is found, we look for these paths in the host scope. If a value is still not found, search the global scope.";

/// Arguments for `config get`.
#[derive(Args)]
pub struct ConfigGetArgs {
    #[arg(help = format!("The configuration path to get. If not provided, get all settings.\n\n{PATH_DEFINITION}"))]
    pub path: Option<String>,

    /// The scope to query.
    /// If not specified, shows the effective value after applying precedence.
    #[arg(long)]
    pub scope: Option<ConfigScope>,

    /// Git remote to use (only relevant for host/remote scopes).
    #[arg(long, default_value = DEFAULT_REMOTE)]
    pub remote: String,
}

/// Arguments for `config set`.
#[derive(Args)]
pub struct ConfigSetArgs {
    #[arg(help = format!("The configuration path to set.\n\n{PATH_DEFINITION}"))]
    pub path: String,

    /// The value to set.
    pub value: String,

    /// The scope to set.
    #[arg(long, default_value = DEFAULT_SET_CMD_SCOPE)]
    pub scope: ConfigScope,

    /// Git remote to use (only relevant for host/remote scopes).
    #[arg(long, default_value = DEFAULT_REMOTE)]
    pub remote: String,
}

/// Arguments for `config unset`.
#[derive(Args)]
pub struct ConfigUnsetArgs {
    #[arg(help = format!("The configuration path to unset.\n\n{PATH_DEFINITION}"))]
    pub path: String,

    /// The scope to unset from.
    #[arg(long, default_value = DEFAULT_SET_CMD_SCOPE)]
    pub scope: ConfigScope,

    /// Git remote to use (only relevant for host/remote scopes).
    #[arg(long, default_value = DEFAULT_REMOTE)]
    pub remote: String,
}

// =============================================================================
// Domain
// =============================================================================

/// Configuration structure stored in TOML format.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// Global settings
    #[serde(flatten)]
    pub global: HashMap<String, String>,

    /// Host-specific settings: key is <host>[:<port>]
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub host: HashMap<String, HashMap<String, String>>,

    /// Remote-specific settings: key is "<host>[:<port>]/<owner>/<repo>"
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub remote: HashMap<String, HashMap<String, String>>,
}

impl Config {
    /// Load configuration from disk.
    pub fn load_from_disk() -> anyhow::Result<Config> {
        confy::load(APP_NAME, CONFIG_NAME).context("Failed to load configuration")
    }

    /// Get a boolean config value.
    pub fn get_bool(&self, path: &str, remote: Option<&GitRemoteData>) -> Option<bool> {
        self.get_value_effective(path, remote).and_then(|(value_str, scope)| {
            value_str.parse::<bool>().ok().or_else(|| {
                eprintln!(
                    "Warning: Invalid boolean value for '{}' in {}: '{}' (expected 'true' or 'false')",
                    path, scope, value_str
                );

                None
            })
        })
    }

    /// Get an enum config value with CLI override using clap's ValueEnum.
    pub fn get_enum<T: ValueEnum>(&self, path: &str, remote: Option<&GitRemoteData>) -> Option<T> {
        self.get_value_effective(path, remote)
            .and_then(|(value_str, scope)| {
                T::from_str(&value_str, true).ok().or_else(|| {
                    let valid_values = T::value_variants()
                        .iter()
                        .filter_map(|v| v.to_possible_value().map(|v| v.get_name().to_string()))
                        .collect::<Vec<_>>()
                        .join(", ");

                    eprintln!(
                        "Warning: Invalid value for '{}' in {}: '{}' (expected one of: {})",
                        path, scope, value_str, valid_values
                    );

                    None
                })
            })
    }

    /// Get a Vec of enums from comma-separated config value.
    pub fn get_enum_vec<T: ValueEnum>(
        &self,
        path: &str,
        remote: Option<&GitRemoteData>,
    ) -> Option<Vec<T>> {
        self.get_value_effective(path, remote).map(|(value_str, scope)| {
            let valid_values = T::value_variants()
                .iter()
                .filter_map(|v| v.to_possible_value().map(|v| v.get_name().to_string()))
                .collect::<Vec<_>>()
                .join(", ");

            value_str
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();

                    T::from_str(trimmed, true).ok().or_else(|| {
                        eprintln!(
                            "Warning: Invalid value '{}' in list for '{}' in {} (expected one of: {})",
                            trimmed, path, scope, valid_values
                        );

                        None
                    })
                })
                .collect()
        })
    }

    /// Get a string config value.
    pub fn get_string(&self, path: &str, remote: Option<&GitRemoteData>) -> Option<String> {
        self.get_value_effective(path, remote).map(|(v, _)| v)
    }

    /// Get a string config value from the global scope.
    pub fn get_string_from_global_scope(&self, path: &str) -> Option<String> {
        self.get_string(path, None)
    }

    /// Get a u32 config value.
    pub fn get_u32(&self, path: &str, remote: Option<&GitRemoteData>) -> Option<u32> {
        self.get_value_effective(path, remote).and_then(|(value_str, scope)| {
            value_str.parse::<u32>().ok().or_else(|| {
                eprintln!(
                    "Warning: Invalid number value for '{}' in {}: '{}' (expected a positive integer)",
                    path, scope, value_str
                );

                None
            })
        })
    }

    /// Get effective value with precedence: remote > host > global.
    fn get_value_effective(
        &self,
        path: &str,
        remote: Option<&GitRemoteData>,
    ) -> Option<(String, ConfigScope)> {
        if let Some(remote) = remote {
            if let Some(value) = self.get_value_from_scope(path, ConfigSource::Remote(remote)) {
                return Some((value, ConfigScope::Remote));
            }

            if let Some(value) = self.get_value_from_scope(path, ConfigSource::Host(remote)) {
                return Some((value, ConfigScope::Host));
            }
        }

        self.get_value_from_scope(path, ConfigSource::Global)
            .map(|value| (value, ConfigScope::Global))
    }

    /// Get value from a specific scope without precedence.
    fn get_value_from_scope(&self, path: &str, source: ConfigSource) -> Option<String> {
        let path_variants = get_path_variants(path);

        match source {
            ConfigSource::Global => {
                for variant in path_variants {
                    if let Some(value) = self.global.get(&variant) {
                        return Some(value.clone());
                    }
                }

                None
            }
            ConfigSource::Host(remote) => {
                let host_key = format_host_key(remote);

                if let Some(host_cfg) = self.host.get(&host_key) {
                    for variant in path_variants {
                        if let Some(value) = host_cfg.get(&variant) {
                            return Some(value.clone());
                        }
                    }
                }

                None
            }
            ConfigSource::Remote(remote) => {
                let remote_key = format_remote_key(remote);

                if let Some(remote_cfg) = self.remote.get(&remote_key) {
                    for variant in path_variants {
                        if let Some(value) = remote_cfg.get(&variant) {
                            return Some(value.clone());
                        }
                    }
                }

                None
            }
        }
    }

    /// Save configuration to disk.
    fn save_to_disk(&self) -> anyhow::Result<()> {
        confy::store(APP_NAME, CONFIG_NAME, self).context("Failed to save configuration")
    }

    /// Set a value in the configuration.
    fn set_value(&mut self, path: &str, value: &str, source: ConfigSource) -> anyhow::Result<()> {
        match source {
            ConfigSource::Global => {
                self.global.insert(path.to_string(), value.to_string());
            }
            ConfigSource::Host(remote) => {
                let host_key = format_host_key(remote);

                self.host
                    .entry(host_key)
                    .or_default()
                    .insert(path.to_string(), value.to_string());
            }
            ConfigSource::Remote(remote) => {
                let remote_key = format_remote_key(remote);

                self.remote
                    .entry(remote_key)
                    .or_default()
                    .insert(path.to_string(), value.to_string());
            }
        }

        Ok(())
    }

    /// Unset a value in the configuration.
    /// Returns true if a value was actually removed, false otherwise.
    fn unset_value(&mut self, path: &str, source: ConfigSource) -> anyhow::Result<bool> {
        let was_removed = match source {
            ConfigSource::Global => self.global.remove(path).is_some(),
            ConfigSource::Host(remote) => {
                let host_key = format_host_key(remote);

                if let Some(host_cfg) = self.host.get_mut(&host_key) {
                    let removed = host_cfg.remove(path).is_some();

                    if host_cfg.is_empty() {
                        self.host.remove(&host_key);
                    }

                    removed
                } else {
                    false
                }
            }
            ConfigSource::Remote(remote) => {
                let remote_key = format_remote_key(remote);

                if let Some(remote_cfg) = self.remote.get_mut(&remote_key) {
                    let removed = remote_cfg.remove(path).is_some();

                    if remote_cfg.is_empty() {
                        self.remote.remove(&remote_key);
                    }

                    removed
                } else {
                    false
                }
            }
        };

        Ok(was_removed)
    }
}

/// Configuration scope.
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum ConfigScope {
    Global,
    Host,
    Remote,
}

impl Display for ConfigScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope_name = match self {
            ConfigScope::Global => "global",
            ConfigScope::Host => "host",
            ConfigScope::Remote => "remote",
        };

        write!(f, "{scope_name} scope")
    }
}

/// Config source that combines the scope and the git remote.
#[derive(Clone, Copy, Debug)]
enum ConfigSource<'a> {
    Global,
    Host(&'a GitRemoteData),
    Remote(&'a GitRemoteData),
}

impl<'a> ConfigSource<'a> {
    fn new(scope: ConfigScope, remote: Option<&'a GitRemoteData>) -> anyhow::Result<Self> {
        match scope {
            ConfigScope::Global => Ok(ConfigSource::Global),
            ConfigScope::Host => {
                let remote = remote.context("Remote data required for host scope")?;

                Ok(ConfigSource::Host(remote))
            }
            ConfigScope::Remote => {
                let remote = remote.context("Remote data required for remote scope")?;

                Ok(ConfigSource::Remote(remote))
            }
        }
    }
}

/// Merges config values into args fields.
///
/// It expects the following arguments (tt):
///   `config`, `args`, optional git `remote`,  base `config path`, `arg fields`
///   in square brackets
///
/// This macro automatically converts field names from `snake_case` to
/// `kebab-case` for the config path.
///
/// # Example
///
/// ```rust,ignore
/// pub fn list_issues(mut args: IssueListCommandArgs) -> anyhow::Result<()> {
///     let (config, remote) = (Config::load_from_disk()?, git::get_remote_data("origin")?);
///
///     merge_config_into_args!(
///         &config,
///         args,
///         Some(&remote),
///         "issue/list",
///         [api, api_url, auth, fields, format, per_page, state, interactive]
///     );
/// }
/// ```
macro_rules! merge_config_into_args {
    ($config:expr, $args:expr, $remote:expr, $command_path:literal, [$($field:ident),* $(,)?]$(,)?) => {
        $(
            {
                let field_name = stringify!($field).replace('_', "-");
                let config_path = format!("{}/{}", $command_path, field_name);

                $crate::cli::config::macro_internals::MergeConfigIntoArg::__merge_with_config(
                    &mut $args.$field,
                    $config,
                    &config_path,
                    $remote,
                );
            }
        )*
    };
}

pub(crate) use merge_config_into_args;

/// Internal module for macro implementation details.
///
/// This module is public only for macro access but hidden from documentation.
pub(crate) mod macro_internals {
    use super::{ApiType, Config, GitRemoteData, IssueState, OutputFormat, PrState};
    use clap::ValueEnum;

    pub trait MergeConfigIntoArg {
        /// Helper function for merging config values into args fields. It isn't
        /// meant to be called manually. Instead use the merge_config_into_args
        /// macro, which calls this function.
        fn __merge_with_config(
            &mut self,
            config: &Config,
            path: &str,
            remote: Option<&GitRemoteData>,
        );
    }

    impl MergeConfigIntoArg for Option<String> {
        fn __merge_with_config(
            &mut self,
            config: &Config,
            path: &str,
            remote: Option<&GitRemoteData>,
        ) {
            if self.is_none() {
                *self = config.get_string(path, remote);
            }
        }
    }

    impl MergeConfigIntoArg for Option<u32> {
        fn __merge_with_config(
            &mut self,
            config: &Config,
            path: &str,
            remote: Option<&GitRemoteData>,
        ) {
            if self.is_none() {
                *self = config.get_u32(path, remote);
            }
        }
    }

    impl MergeConfigIntoArg for bool {
        fn __merge_with_config(
            &mut self,
            config: &Config,
            path: &str,
            remote: Option<&GitRemoteData>,
        ) {
            if !*self {
                *self = config.get_bool(path, remote).unwrap_or_default();
            }
        }
    }

    impl<T: ValueEnum> MergeConfigIntoArg for Vec<T> {
        fn __merge_with_config(
            &mut self,
            config: &Config,
            path: &str,
            remote: Option<&GitRemoteData>,
        ) {
            if self.is_empty() {
                *self = config.get_enum_vec(path, remote).unwrap_or_default();
            }
        }
    }

    macro_rules! impl_merge_from_config_for_enum {
        ($enum_type:ty) => {
            impl MergeConfigIntoArg for Option<$enum_type> {
                fn __merge_with_config(
                    &mut self,
                    config: &Config,
                    path: &str,
                    remote: Option<&GitRemoteData>,
                ) {
                    if self.is_none() {
                        *self = config.get_enum(path, remote);
                    }
                }
            }
        };
    }

    impl_merge_from_config_for_enum!(ApiType);
    impl_merge_from_config_for_enum!(OutputFormat);
    impl_merge_from_config_for_enum!(IssueState);
    impl_merge_from_config_for_enum!(PrState);
}

// =============================================================================
// Command Logic
// =============================================================================

/// Execute the `config get` subcommand.
pub fn config_get(args: ConfigGetArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;

    match args.path {
        Some(path) => match args.scope {
            Some(scope) => {
                let remote = get_remote_for_scope(&scope, &args.remote)?;
                let source = ConfigSource::new(scope, remote.as_ref())?;

                match config.get_value_from_scope(&path, source) {
                    Some(value) => println!("{value}"),
                    None => eprintln!("No value found for '{path}' in {scope}"),
                }
            }
            None => {
                // If there is no scope, fall back to global scope.
                let remote = git::get_remote_data(&args.remote).ok();

                match config.get_value_effective(&path, remote.as_ref()) {
                    Some((value, _)) => println!("{value}"),
                    None => eprintln!("No value found for '{path}'"),
                }
            }
        },
        None => match args.scope {
            Some(scope) => {
                let remote = get_remote_for_scope(&scope, &args.remote)?;
                let source = ConfigSource::new(scope, remote.as_ref())?;

                print_entire_config_for_scope(&config, source)?;
            }
            None => {
                // If there is no scope, show global config.
                let remote = git::get_remote_data(&args.remote).ok();

                print_entire_effective_config(&config, remote.as_ref())?;
            }
        },
    };

    Ok(())
}

/// Execute the `config set` subcommand.
pub fn config_set(args: ConfigSetArgs) -> anyhow::Result<()> {
    let mut config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote = get_remote_for_scope(&args.scope, &args.remote)?;
    let source = ConfigSource::new(args.scope, remote.as_ref())?;

    config.set_value(&args.path, &args.value, source)?;
    config.save_to_disk()?;

    Ok(())
}

/// Execute the `config unset` subcommand.
pub fn config_unset(args: ConfigUnsetArgs) -> anyhow::Result<()> {
    let mut config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote = get_remote_for_scope(&args.scope, &args.remote)?;
    let source = ConfigSource::new(args.scope, remote.as_ref())?;
    let was_removed = config.unset_value(&args.path, source)?;

    if was_removed {
        config.save_to_disk()?;
        println!("Unset '{}' from {}", args.path, args.scope);
    } else {
        eprintln!("No value found for '{}' in {}", args.path, args.scope);
    }

    Ok(())
}

/// Execute the `config edit` subcommand.
pub fn config_edit() -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let mut editor = Editor::new();

    if let Some(cmd) = config.get_string_from_global_scope("editor-command") {
        editor.executable(cmd);
    };

    let config_path = match confy::get_configuration_file_path(APP_NAME, CONFIG_NAME) {
        Ok(path) => path,
        Err(e) => anyhow::bail!("Failed to get config path: {}", e),
    };
    let edited_content = editor
        .edit(&fs::read_to_string(&config_path).unwrap_or_default())
        .context("Failed to open editor")?;

    if let Some(content) = edited_content {
        fs::write(&config_path, content.as_bytes())
            .context("Failed to write configuration file")?;

        Config::load_from_disk()
            .context("The config file may be corrupted. Please check the TOML file.")?;

        println!("Configuration saved successfully.");
    }

    Ok(())
}

// =============================================================================
// Private Helpers
// =============================================================================

/// Format a remote identifier for use as a config key.
fn format_remote_key(remote: &GitRemoteData) -> String {
    if let Some(port) = remote.port {
        format!("{}:{}/{}", remote.host, port, remote.path)
    } else {
        format!("{}/{}", remote.host, remote.path)
    }
}

/// Format a host identifier for use as a config key.
fn format_host_key(remote: &GitRemoteData) -> String {
    if let Some(port) = remote.port {
        format!("{}:{}", remote.host, port)
    } else {
        remote.host.clone()
    }
}

/// Get the applicable path variants for a given (full) path by walking up the
/// command path hierarchy.
///
/// A path has the format `[<COMMAND_PATH>/]<FLAG>`, where:
/// - `<COMMAND_PATH>` is a slash-separated path of commmands, e.g. `pr/create`
/// - `<FLAG>` is the flag of the command, e.g. `editor`
///
/// The variants are generated by progressively removing levels from the end of
/// the command path while keeping the flag constant.
///
/// # Examples
///
/// - `pr/create/editor` → `["pr/create/editor", "pr/editor", "editor"]`
/// - `pr/editor` → `["pr/editor", "editor"]`
/// - `editor` → `["editor"]`
fn get_path_variants(path: &str) -> Vec<String> {
    let parts: Vec<&str> = path.split('/').collect();

    if parts.is_empty() {
        return Vec::new();
    }

    let flag_index = parts.len() - 1;
    let flag = parts[flag_index];
    let command_path_parts = &parts[..flag_index];

    if command_path_parts.is_empty() {
        return vec![flag.to_string()];
    }

    let mut variants = vec![path.to_string()];

    for i in (1..command_path_parts.len()).rev() {
        let truncated_path = command_path_parts[..i].join("/");

        variants.push(format!("{}/{}", truncated_path, flag));
    }

    variants.push(flag.to_string());

    variants
}

/// Gets the git remote for a given scope.
fn get_remote_for_scope(
    scope: &ConfigScope,
    remote_name: &str,
) -> anyhow::Result<Option<GitRemoteData>> {
    let remote = match scope {
        ConfigScope::Global => None,
        ConfigScope::Host | ConfigScope::Remote => {
            Some(git::get_remote_data(remote_name).with_context(|| {
                format!("Failed to get remote URL for remote '{}'", remote_name)
            })?)
        }
    };

    Ok(remote)
}

/// List values from a specific scope.
fn print_entire_config_for_scope(config: &Config, source: ConfigSource) -> anyhow::Result<()> {
    match source {
        ConfigSource::Global => {
            let mut sorted_entries: Vec<_> = config.global.iter().collect();
            sorted_entries.sort_by_key(|(k, _)| *k);

            for (key, value) in sorted_entries {
                println!("{} = {}", key, value);
            }
        }
        ConfigSource::Host(remote) => {
            let host_key = format_host_key(remote);

            if let Some(host_cfg) = config.host.get(&host_key) {
                let mut sorted_entries: Vec<_> = host_cfg.iter().collect();
                sorted_entries.sort_by_key(|(k, _)| *k);

                for (key, value) in sorted_entries {
                    println!("{} = {}", key, value);
                }
            }
        }
        ConfigSource::Remote(remote) => {
            let remote_key = format_remote_key(remote);

            if let Some(remote_cfg) = config.remote.get(&remote_key) {
                let mut sorted_entries: Vec<_> = remote_cfg.iter().collect();
                sorted_entries.sort_by_key(|(k, _)| *k);

                for (key, value) in sorted_entries {
                    println!("{} = {}", key, value);
                }
            }
        }
    }

    Ok(())
}

/// Print effective configuration with precedence applied.
fn print_entire_effective_config(
    config: &Config,
    remote: Option<&GitRemoteData>,
) -> anyhow::Result<()> {
    let mut all_paths = HashSet::new();

    // global
    for key in config.global.keys() {
        all_paths.insert(key);
    }

    if let Some(remote) = remote {
        // host
        let host_key = format_host_key(remote);

        if let Some(host_cfg) = config.host.get(&host_key) {
            for key in host_cfg.keys() {
                all_paths.insert(key);
            }
        }

        // remote
        let remote_key = format_remote_key(remote);

        if let Some(remote_cfg) = config.remote.get(&remote_key) {
            for key in remote_cfg.keys() {
                all_paths.insert(key);
            }
        }
    }

    let mut sorted_paths: Vec<_> = all_paths.into_iter().collect();
    sorted_paths.sort();

    for path in sorted_paths {
        if let Some((value, scope)) = config.get_value_effective(path, remote) {
            println!("{path} = {value} ({scope})");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_git_remote(host: &str, path: &str, port: Option<u16>) -> GitRemoteData {
        GitRemoteData {
            host: host.to_string(),
            path: path.to_string(),
            port,
        }
    }

    // =========================================================================
    // get_path_variants
    // =========================================================================

    #[test]
    fn test_get_path_variants_simple() {
        let variants = get_path_variants("editor");
        assert_eq!(variants, vec!["editor"]);
    }

    #[test]
    fn test_get_path_variants_two_levels() {
        let variants = get_path_variants("pr/editor");
        assert_eq!(variants, vec!["pr/editor", "editor"]);
    }

    #[test]
    fn test_get_path_variants_three_levels() {
        let variants = get_path_variants("pr/create/editor");
        assert_eq!(variants, vec!["pr/create/editor", "pr/editor", "editor"]);
    }

    #[test]
    fn test_get_path_variants_four_levels() {
        let variants = get_path_variants("foo/bar/baz/baz");
        assert_eq!(
            variants,
            vec!["foo/bar/baz/baz", "foo/bar/baz", "foo/baz", "baz"]
        );
    }

    // =========================================================================
    // Scope Precedence
    // =========================================================================

    #[test]
    fn test_scope_precedence_remote_wins() {
        let mut config = Config::default();
        let remote = create_git_remote("github.com", "user/repo", None);

        config
            .global
            .insert("editor".to_string(), "global-vim".to_string());
        config
            .host
            .entry("github.com".to_string())
            .or_default()
            .insert("editor".to_string(), "host-nano".to_string());
        config
            .remote
            .entry("github.com/user/repo".to_string())
            .or_default()
            .insert("editor".to_string(), "remote-emacs".to_string());

        let (value, scope) = config.get_value_effective("editor", Some(&remote)).unwrap();
        assert_eq!(value, "remote-emacs");
        assert_eq!(scope, ConfigScope::Remote);
    }

    #[test]
    fn test_scope_precedence_host_wins_when_no_remote() {
        let mut config = Config::default();
        let remote = create_git_remote("github.com", "user/repo", None);

        config
            .global
            .insert("editor".to_string(), "global-vim".to_string());
        config
            .host
            .entry("github.com".to_string())
            .or_default()
            .insert("editor".to_string(), "host-nano".to_string());

        let (value, scope) = config.get_value_effective("editor", Some(&remote)).unwrap();
        assert_eq!(value, "host-nano");
        assert_eq!(scope, ConfigScope::Host);
    }

    #[test]
    fn test_scope_precedence_global_wins_when_no_host_or_remote() {
        let mut config = Config::default();
        let remote = create_git_remote("github.com", "user/repo", None);

        config
            .global
            .insert("editor".to_string(), "global-vim".to_string());

        let (value, scope) = config.get_value_effective("editor", Some(&remote)).unwrap();
        assert_eq!(value, "global-vim");
        assert_eq!(scope, ConfigScope::Global);
    }

    #[test]
    fn test_scope_precedence_no_remote_context() {
        let mut config = Config::default();

        config
            .global
            .insert("editor".to_string(), "global-vim".to_string());
        config
            .host
            .entry("github.com".to_string())
            .or_default()
            .insert("editor".to_string(), "host-nano".to_string());

        let (value, scope) = config.get_value_effective("editor", None).unwrap();
        assert_eq!(value, "global-vim");
        assert_eq!(scope, ConfigScope::Global);
    }

    #[test]
    fn test_scope_precedence_path_hierarchy() {
        let mut config = Config::default();
        let remote = create_git_remote("github.com", "user/repo", None);

        config
            .global
            .insert("pr/create/editor".to_string(), "specific".to_string());
        config
            .global
            .insert("editor".to_string(), "general".to_string());

        let (value, _) = config
            .get_value_effective("pr/create/editor", Some(&remote))
            .unwrap();
        assert_eq!(value, "specific");

        let (value, _) = config
            .get_value_effective("pr/list/editor", Some(&remote))
            .unwrap();
        assert_eq!(value, "general");
    }

    #[test]
    fn test_get_bool_true() {
        let mut config = Config::default();

        config.global.insert("flag".to_string(), "true".to_string());

        assert_eq!(config.get_bool("flag", None), Some(true));
    }

    #[test]
    fn test_get_bool_false() {
        let mut config = Config::default();

        config
            .global
            .insert("flag".to_string(), "false".to_string());

        assert_eq!(config.get_bool("flag", None), Some(false));
    }

    #[test]
    fn test_get_bool_invalid_values() {
        let mut config = Config::default();
        let invalid_values = vec!["1", "0", "yes", "no", "True", "False", "invalid"];

        for value in invalid_values {
            config.global.insert("flag".to_string(), value.to_string());
            assert_eq!(
                config.get_bool("flag", None),
                None,
                "Expected None for value: {}",
                value
            );
        }
    }

    #[test]
    fn test_get_u32_valid() {
        let mut config = Config::default();
        config.global.insert("count".to_string(), "42".to_string());

        assert_eq!(config.get_u32("count", None), Some(42));
    }

    #[test]
    fn test_get_u32_text_returns_none() {
        let mut config = Config::default();
        config.global.insert("count".to_string(), "abc".to_string());

        assert_eq!(config.get_u32("count", None), None);
    }

    #[test]
    fn test_get_string() {
        let mut config = Config::default();

        config
            .global
            .insert("editor".to_string(), "vim".to_string());

        assert_eq!(
            config.get_string_from_global_scope("editor"),
            Some("vim".to_string())
        );
        assert_eq!(config.get_string("nonexistent", None), None);
    }

    #[test]
    fn test_get_enum_all_variants() {
        let mut config = Config::default();

        config
            .global
            .insert("scope".to_string(), "global".to_string());

        assert_eq!(
            config.get_enum::<ConfigScope>("scope", None).unwrap(),
            ConfigScope::Global
        );

        config
            .global
            .insert("scope".to_string(), "host".to_string());

        assert_eq!(
            config.get_enum::<ConfigScope>("scope", None).unwrap(),
            ConfigScope::Host
        );

        config
            .global
            .insert("scope".to_string(), "remote".to_string());

        assert_eq!(
            config.get_enum::<ConfigScope>("scope", None).unwrap(),
            ConfigScope::Remote
        );
    }

    #[test]
    fn test_get_enum_vec() {
        let mut config = Config::default();
        config
            .global
            .insert("scopes".to_string(), "global,host, remote".to_string());

        let result = config.get_enum_vec::<ConfigScope>("scopes", None).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ConfigScope::Global);
        assert_eq!(result[1], ConfigScope::Host);
        assert_eq!(result[2], ConfigScope::Remote);
    }

    #[test]
    fn test_get_enum_vec_filters_invalid() {
        let mut config = Config::default();
        config
            .global
            .insert("scopes".to_string(), "global,invalid,host".to_string());

        let result = config.get_enum_vec::<ConfigScope>("scopes", None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ConfigScope::Global);
        assert_eq!(result[1], ConfigScope::Host);
    }
}
