use anyhow::Context;
use dialoguer::Editor;

#[derive(Debug)]
pub struct Message {
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

pub fn open_text_editor_to_write_message() -> anyhow::Result<Message> {
    let Some(file_content) = Editor::new()
        .edit(MESSAGE_TEMPLATE)
        .context("Failed opening text editor to enter message")?
    else {
        anyhow::bail!("Aborting due to missing message.")
    };

    let (title, body) =
        match file_content.rsplit_once("# ------------------------ >8 ------------------------") {
            Some((content, _)) => match content.split_once("\n") {
                Some((title, body)) => (title.trim(), body.trim()),
                None => (content.trim(), ""),
            },
            None => anyhow::bail!("The cut marker has been removed. Aborting..."),
        };

    Ok(Message {
        body: body.to_string(),
        title: title.to_string(),
    })
}
