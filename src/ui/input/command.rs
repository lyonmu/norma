use gpui::Keystroke;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyBindingContext {
    TextField,
    TextArea,
    Composer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputCommand {
    InsertText(String),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveToStart { selecting: bool },
    MoveToEnd { selecting: bool },
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    Submit,
    Blur,
}

pub fn key_to_command(
    keystroke: &Keystroke,
    context: KeyBindingContext,
) -> Option<InputCommand> {
    let modifiers = keystroke.modifiers;
    let secondary = modifiers.secondary();
    let shift = modifiers.shift;
    let only_shift = shift && !modifiers.secondary() && !modifiers.control && !modifiers.alt;
    let no_modifiers = !modifiers.modified();

    if secondary {
        match keystroke.key.as_str() {
            "a" => return Some(InputCommand::SelectAll),
            "c" => return Some(InputCommand::Copy),
            "x" => return Some(InputCommand::Cut),
            "v" => return Some(InputCommand::Paste),
            "z" if shift => return Some(InputCommand::Redo),
            "z" => return Some(InputCommand::Undo),
            _ => {}
        }
    }

    match keystroke.key.as_str() {
        "backspace" => Some(InputCommand::DeleteBackward),
        "delete" => Some(InputCommand::DeleteForward),
        "left" => Some(InputCommand::MoveLeft { selecting: shift }),
        "right" => Some(InputCommand::MoveRight { selecting: shift }),
        "home" => Some(InputCommand::MoveToStart { selecting: shift }),
        "end" => Some(InputCommand::MoveToEnd { selecting: shift }),
        "escape" => Some(InputCommand::Blur),
        "enter" => match context {
            KeyBindingContext::Composer if shift => Some(InputCommand::InsertNewline),
            KeyBindingContext::Composer => Some(InputCommand::Submit),
            KeyBindingContext::TextArea => Some(InputCommand::InsertNewline),
            KeyBindingContext::TextField => Some(InputCommand::Submit),
        },
        _ if no_modifiers || only_shift => keystroke
            .key_char
            .as_ref()
            .map(|text| InputCommand::InsertText(text.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use gpui::Keystroke;

    fn key(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            key: key.to_string(),
            key_char: key_char.map(str::to_string),
            modifiers,
        }
    }

    #[test]
    fn maps_plain_character_to_insert_text() {
        assert_eq!(
            key_to_command(&key("a", Some("a"), Modifiers::none()), KeyBindingContext::TextField),
            Some(InputCommand::InsertText("a".to_string()))
        );
    }

    #[test]
    fn maps_secondary_shortcuts() {
        assert_eq!(
            key_to_command(&key("a", None, Modifiers::secondary_key()), KeyBindingContext::TextField),
            Some(InputCommand::SelectAll)
        );
        assert_eq!(
            key_to_command(&key("z", None, Modifiers::secondary_key()), KeyBindingContext::TextField),
            Some(InputCommand::Undo)
        );
        assert_eq!(
            key_to_command(
                &key("z", None, Modifiers {
                    shift: true,
                    ..Modifiers::secondary_key()
                }),
                KeyBindingContext::TextField
            ),
            Some(InputCommand::Redo)
        );
    }

    #[test]
    fn maps_delete_and_movement() {
        assert_eq!(
            key_to_command(&key("backspace", None, Modifiers::none()), KeyBindingContext::TextField),
            Some(InputCommand::DeleteBackward)
        );
        assert_eq!(
            key_to_command(&key("delete", None, Modifiers::none()), KeyBindingContext::TextField),
            Some(InputCommand::DeleteForward)
        );
        assert_eq!(
            key_to_command(&key("left", None, Modifiers::shift()), KeyBindingContext::TextField),
            Some(InputCommand::MoveLeft { selecting: true })
        );
    }

    #[test]
    fn enter_submits_in_composer_and_shift_enter_inserts_newline() {
        assert_eq!(
            key_to_command(&key("enter", None, Modifiers::none()), KeyBindingContext::Composer),
            Some(InputCommand::Submit)
        );
        assert_eq!(
            key_to_command(&key("enter", None, Modifiers::shift()), KeyBindingContext::Composer),
            Some(InputCommand::InsertNewline)
        );
    }
}
