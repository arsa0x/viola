use std::collections::BTreeMap;
use viola_core::{
    command::COMMANDS,
    context::Context,
    message::interactive::single_select::{SingleSelectRow, SingleSelectSection},
};
use viola_macros::command;

#[command(
    triggers = ["menu", "help"],
    description = "show bot menu",
    category = "tools"
)]
async fn menu(ctx: Context) -> anyhow::Result<()> {
    let mut categories: BTreeMap<&str, Vec<&viola_core::command::Command>> = BTreeMap::new();

    for command in COMMANDS {
        categories
            .entry(command.category)
            .or_default()
            .push(command);
    }

    let sections = categories
        .into_iter()
        .map(|(category, commands)| SingleSelectSection {
            title: category.to_string(),

            rows: commands
                .into_iter()
                .map(|cmd| SingleSelectRow {
                    title: humanize_command(cmd.name),

                    description: if cmd.description.is_empty() {
                        "No description".into()
                    } else {
                        cmd.description.to_string()
                    },
                    id: format!(".{} --help", cmd.triggers[0]),
                })
                .collect(),
        })
        .collect();

    ctx.send()
        .interactive()
        .single_select(sections)
        .title("Viola Bot Menu")
        .text_body("Select the command you want to run")
        .select_label("Open Menu")
        .quoted()
        .await?;

    Ok(())
}

fn humanize_command(name: &str) -> String {
    let text = name.replace('_', " ");

    let mut chars = text.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
