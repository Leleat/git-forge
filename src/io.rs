//! Input/Output utilities.

use anyhow::Context;
use clap::ValueEnum;
use csv::WriterBuilder;
use dialoguer::Editor;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
pub struct InputMessage {
    pub title: String,
    pub body: String,
}

const MESSAGE_TEMPLATE: &str = "

# ------------------------ >8 ------------------------
# Do not modify or remove the line above.
# Everything below it will be ignored.

## Help

Enter a message above the cut marker (the line containing -- >8 --).
The first line of your message will be used as the title.
The remaining text will be used for the description.
Save and exit your editor to continue.

## Example

```txt
This line will be used as the title

The description starts with this line. It can contain
multiple paragraphs.

That means, this line will also be part of the description.

# --- <cut marker> ---
# ...
```
";

/// Opens the default text editor for the user to write a message. The first
/// line of will be used as the title while the rest will be used for the body.
pub fn prompt_with_default_text_editor() -> anyhow::Result<InputMessage> {
    prompt_with_text_editor(None)
}

/// Opens the a custom text editor with the provided command for the user to
/// write a message. The first line of will be used as the title while the rest
/// will be used for the body.
pub fn prompt_with_custom_text_editor(cmd: &str) -> anyhow::Result<InputMessage> {
    prompt_with_text_editor(Some(cmd))
}

fn prompt_with_text_editor(cmd: Option<&str>) -> anyhow::Result<InputMessage> {
    let mut editor = Editor::new();

    if let Some(exec) = cmd {
        editor.executable(exec);
    }

    let Some(file_content) = editor
        .edit(MESSAGE_TEMPLATE)
        .context("Failed opening text editor to enter message")?
    else {
        anyhow::bail!("Aborting: No message provided (editor closed without saving)")
    };

    let (title, body) = match file_content
        .rsplit_once("# ------------------------ >8 ------------------------")
    {
        Some((content, _)) => match content.split_once("\n") {
            Some((title, body)) => (title.trim(), body.trim()),
            None => (content.trim(), ""),
        },
        None => anyhow::bail!(
            "The cut marker '# ------------------------ >8 ------------------------' was removed or modified. \
                 This marker is required to separate your message from the help text."
        ),
    };

    Ok(InputMessage {
        body: body.to_string(),
        title: title.to_string(),
    })
}

/// Output format.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Comma-separated values format.
    Csv,
    /// Tab-separated values format.
    #[default]
    Tsv,
    /// JSON format.
    Json,
}

/// Format a collection of items using the specified output format.
pub fn format<T, F>(items: &[T], fields: &[F], format: &OutputFormat) -> anyhow::Result<String>
where
    T: Serialize,
    F: Serialize,
{
    match format {
        OutputFormat::Tsv => format_delimited(items, fields, b'\t'),
        OutputFormat::Csv => format_delimited(items, fields, b','),
        OutputFormat::Json => format_json(items, fields),
    }
}

fn format_json<T, F>(items: &[T], fields: &[F]) -> anyhow::Result<String>
where
    T: Serialize,
    F: Serialize,
{
    let json_values: Vec<Value> = items
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()?;

    if fields.is_empty() {
        return Ok(serde_json::to_string_pretty(&json_values)?);
    }

    let field_names = get_field_names(fields);
    let filtered_items = json_values
        .into_iter()
        .map(|mut item| {
            if let Value::Object(ref mut map) = item {
                map.retain(|key, _| field_names.iter().any(|name| name == key));
            }

            item
        })
        .collect::<Vec<Value>>();

    Ok(serde_json::to_string_pretty(&filtered_items)?)
}

fn format_delimited<T, F>(items: &[T], fields: &[F], delimiter: u8) -> anyhow::Result<String>
where
    T: Serialize,
    F: Serialize,
{
    let mut writer = WriterBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .from_writer(vec![]);
    let field_names = get_field_names(fields);

    for item in items {
        let json_value = serde_json::to_value(item)?;
        let record = match json_value {
            Value::Object(map) if field_names.is_empty() => map
                .values()
                .map(stringify_json_value_for_serialization)
                .collect(),
            Value::Object(map) => field_names
                .iter()
                .map(|field_name| {
                    map.get(field_name)
                        .map(stringify_json_value_for_serialization)
                        .unwrap_or_default()
                })
                .collect(),
            _ => vec![stringify_json_value_for_serialization(&json_value)],
        };

        writer.write_record(&record)?;
    }

    let bytes = writer.into_inner()?;
    let mut output = String::from_utf8(bytes)?;

    if output.ends_with("\n") {
        output.pop();
    }

    Ok(output)
}

fn stringify_json_value_for_serialization(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => stringify_json_value_for_serialization(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        // Hmm, not sure about falling back to json...
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn get_field_names<T: Serialize>(fields: &[T]) -> Vec<String> {
    fields
        .iter()
        .filter_map(|f| match serde_json::to_value(f) {
            Ok(Value::String(s)) => Some(s),
            _ => None,
        })
        .collect::<Vec<String>>()
}
