pub fn prompt_input<S: ToString>(
    prompt_message: &str,
    cli_arg_value: &Option<S>,
    interactive: bool,
    allow_empty: bool,
) -> String {
    match cli_arg_value {
        Some(value) => value.to_string(),
        None => {
            if interactive {
                dialoguer::Input::<String>::new()
                    .with_prompt(prompt_message)
                    .allow_empty(allow_empty)
                    .show_default(false)
                    .interact()
                    .unwrap_or_default()
            } else {
                panic!("CLI argument required: {}", prompt_message)
            }
        }
    }
}

pub fn prompt_bool(prompt_message: &str, cli_arg_value: Option<bool>, interactive: bool) -> bool {
    match cli_arg_value {
        Some(value) => value,
        None => {
            if interactive {
                dialoguer::Confirm::new()
                    .with_prompt(prompt_message)
                    .default(false)
                    .show_default(true)
                    .interact()
                    .unwrap_or_default()
            } else {
                panic!("CLI argument required: {}", prompt_message)
            }
        }
    }
}

pub fn prompt_editor(prompt_message: &str, interactive: bool) -> String {
    if interactive {
        match dialoguer::Editor::new().edit(prompt_message) {
            Ok(Some(content)) => content,
            Ok(None) => {
                println!("No input received.");
                String::new()
            }
            Err(err) => {
                eprintln!("Error opening editor: {}", err);
                String::new()
            }
        }
    } else {
        panic!("CLI argument required: {}", prompt_message);
    }
}
