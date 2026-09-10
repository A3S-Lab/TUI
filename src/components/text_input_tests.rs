use super::*;
use crossterm::event::KeyModifiers;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
    }
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
    }
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::ALT,
    }
}

#[test]
fn typing_characters() {
    let mut input = TextInput::new();
    input.handle_key(&key(KeyCode::Char('h')));
    input.handle_key(&key(KeyCode::Char('i')));
    assert_eq!(input.value(), "hi");
}

#[test]
fn backspace_deletes() {
    let mut input = TextInput::new();
    input.handle_key(&key(KeyCode::Char('a')));
    input.handle_key(&key(KeyCode::Char('b')));
    input.handle_key(&key(KeyCode::Backspace));
    assert_eq!(input.value(), "a");
}

#[test]
fn cursor_movement() {
    let mut input = TextInput::new();
    input.set_value("hello");
    input.handle_key(&key(KeyCode::Home));
    assert_eq!(input.cursor, 0);
    input.handle_key(&key(KeyCode::End));
    assert_eq!(input.cursor, 5);
    input.handle_key(&key(KeyCode::Left));
    assert_eq!(input.cursor, 4);
    input.handle_key(&key(KeyCode::Right));
    assert_eq!(input.cursor, 5);
}

#[test]
fn char_limit() {
    let mut input = TextInput::new().with_char_limit(3);
    input.handle_key(&key(KeyCode::Char('a')));
    input.handle_key(&key(KeyCode::Char('b')));
    input.handle_key(&key(KeyCode::Char('c')));
    input.handle_key(&key(KeyCode::Char('d')));
    assert_eq!(input.value(), "abc");
}

#[test]
fn handle_event_paste_sanitizes_single_line_text() {
    let mut input = TextInput::new();

    let msg = input.handle_event(&Event::Paste("hello\r\nworld\t!\u{7}".to_string()));

    assert!(matches!(msg, Some(TextInputMsg::Changed(value)) if value == "hello world !"));
    assert_eq!(input.value(), "hello world !");
}

#[test]
fn paste_respects_char_limit_at_cursor() {
    let mut input = TextInput::new().with_char_limit(4);
    input.set_value("ab");
    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&key(KeyCode::Right));

    input.insert_str("XYZ");

    assert_eq!(input.value(), "aXYb");
    assert_eq!(input.cursor(), 3);
}

#[test]
fn control_modified_characters_are_not_text_input() {
    let mut input = TextInput::new();

    assert!(input.handle_key(&ctrl(KeyCode::Char('c'))).is_none());

    assert_eq!(input.value(), "");
}

#[test]
fn word_navigation_and_deletion() {
    let mut input = TextInput::new();
    input.set_value("hello brave world");

    input.handle_key(&alt(KeyCode::Char('b')));
    assert_eq!(input.cursor(), 12);

    input.handle_key(&alt(KeyCode::Char('d')));
    assert_eq!(input.value(), "hello brave ");
    assert_eq!(input.cursor(), 12);

    input.handle_key(&ctrl(KeyCode::Char('w')));
    assert_eq!(input.value(), "hello ");
    assert_eq!(input.cursor(), 6);

    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&alt(KeyCode::Char('d')));
    assert_eq!(input.value(), "");
    assert_eq!(input.cursor(), 0);
}

#[test]
fn char_limit_counts_multibyte_chars() {
    let mut input = TextInput::new().with_char_limit(2);
    input.handle_key(&key(KeyCode::Char('你')));
    input.handle_key(&key(KeyCode::Char('好')));
    input.handle_key(&key(KeyCode::Char('a')));

    assert_eq!(input.value(), "你好");
}

#[test]
fn set_value_honors_char_limit_on_char_boundaries() {
    let mut input = TextInput::new().with_char_limit(2);
    input.set_value("你好abc");

    assert_eq!(input.value(), "你好");
    assert_eq!(input.cursor, 2);
}

#[test]
fn set_value_honors_zero_char_limit() {
    let mut input = TextInput::new().with_char_limit(0);
    input.set_value("hello");

    assert_eq!(input.value(), "");
    assert_eq!(input.cursor, 0);
}

#[test]
fn multibyte_input_edits_on_char_boundaries() {
    let mut input = TextInput::new();
    for ch in "你好abc".chars() {
        input.handle_key(&key(KeyCode::Char(ch)));
    }

    for _ in 0..3 {
        input.handle_key(&key(KeyCode::Left));
    }
    input.handle_key(&key(KeyCode::Backspace));
    input.handle_key(&key(KeyCode::Delete));

    assert_eq!(input.value(), "你bc");
}

#[test]
fn multibyte_end_cursor_renders_after_value() {
    let mut input = TextInput::new();
    input.set_value("你好");

    assert!(input.view().ends_with("\x1b[7m \x1b[0m"));

    let Element::Box(row) = input.element::<()>() else {
        panic!("expected row element");
    };
    let Element::Text(cursor) = row.children.last().expect("expected cursor child") else {
        panic!("expected cursor text");
    };
    assert_eq!(cursor.content, " ");
    assert!(cursor.style.reverse);
}

#[test]
fn element_uses_structured_cursor_style() {
    let mut input = TextInput::new();
    input.set_value("abc");
    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&key(KeyCode::Right));

    let Element::Box(row) = input.element::<()>() else {
        panic!("expected row element");
    };
    assert_eq!(row.children.len(), 3);
    let Element::Text(cursor) = &row.children[1] else {
        panic!("expected cursor text");
    };
    assert_eq!(cursor.content, "b");
    assert!(cursor.style.reverse);
    assert!(!cursor.content.contains('\x1b'));
}

#[test]
fn focused_placeholder_keeps_cursor_visible() {
    let input = TextInput::new()
        .with_prefix("> ")
        .with_placeholder("command");

    let view = input.view();

    assert_eq!(crate::style::strip_ansi(&view), "> command");
    assert!(view.contains("\x1b[2;7mc\x1b[0m"));

    let Element::Box(row) = input.element::<()>() else {
        panic!("expected row element");
    };
    assert_eq!(row.children.len(), 3);
    let Element::Text(cursor) = &row.children[1] else {
        panic!("expected cursor text");
    };
    assert_eq!(cursor.content, "c");
    assert!(cursor.style.reverse);
    assert!(cursor.style.dim);
}

#[test]
fn cursor_styles_following_zero_width_marks_with_base_glyph() {
    let mut input = TextInput::new();
    input.set_value("e\u{301}x");
    input.handle_key(&key(KeyCode::Home));

    assert_eq!(crate::style::strip_ansi(&input.view()), "e\u{301}x");
    assert!(input.view().contains("\x1b[7me\u{301}\x1b[0mx"));

    let Element::Box(row) = input.element::<()>() else {
        panic!("expected row element");
    };
    let Element::Text(cursor) = &row.children[0] else {
        panic!("expected cursor text");
    };
    assert_eq!(cursor.content, "e\u{301}");
    assert!(cursor.style.reverse);
}

#[test]
fn cursor_inside_zero_width_span_styles_base_glyph() {
    let mut input = TextInput::new();
    input.set_value("e\u{301}x");
    input.cursor = 1;

    assert_eq!(input.cursor, 1);
    assert_eq!(crate::style::strip_ansi(&input.view()), "e\u{301}x");
    assert!(input.view().contains("\x1b[7me\u{301}\x1b[0mx"));

    let Element::Box(row) = input.element::<()>() else {
        panic!("expected row element");
    };
    let Element::Text(cursor) = &row.children[0] else {
        panic!("expected cursor text");
    };
    assert_eq!(cursor.content, "e\u{301}");
    assert!(cursor.style.reverse);
}

#[test]
fn cursor_movement_skips_zero_width_marks() {
    let mut input = TextInput::new();
    input.set_value("e\u{301}x");
    input.handle_key(&key(KeyCode::Home));

    input.handle_key(&key(KeyCode::Right));
    assert_eq!(input.cursor, 2);
    assert!(input.view().contains("e\u{301}\x1b[7mx\x1b[0m"));

    input.handle_key(&key(KeyCode::Right));
    assert_eq!(input.cursor, 3);

    input.handle_key(&key(KeyCode::Left));
    assert_eq!(input.cursor, 2);
    input.handle_key(&key(KeyCode::Left));
    assert_eq!(input.cursor, 0);
}

#[test]
fn edit_keys_remove_zero_width_span_with_base_glyph() {
    let mut input = TextInput::new();
    input.set_value("e\u{301}x");
    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&key(KeyCode::Delete));

    assert_eq!(input.value(), "x");
    assert_eq!(input.cursor, 0);

    input.set_value("e\u{301}x");
    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&key(KeyCode::Right));
    input.handle_key(&key(KeyCode::Backspace));

    assert_eq!(input.value(), "x");
    assert_eq!(input.cursor, 0);
}

#[test]
fn editing_normalizes_stale_cursor() {
    let mut input = TextInput::new();
    input.set_value("ab");
    input.cursor = usize::MAX;

    input.handle_key(&key(KeyCode::Char('!')));

    assert_eq!(input.value(), "ab!");
    assert_eq!(input.cursor, 3);
}

#[test]
fn rendering_normalizes_stale_cursor() {
    let mut input = TextInput::new();
    input.set_value("ab");
    input.cursor = usize::MAX;

    assert!(input.view().ends_with("\x1b[7m \x1b[0m"));

    let Element::Box(row) = input.element::<()>() else {
        panic!("expected row element");
    };
    let Element::Text(cursor) = row.children.last().expect("expected cursor child") else {
        panic!("expected cursor text");
    };
    assert_eq!(cursor.content, " ");
    assert!(cursor.style.reverse);
}

#[test]
fn submit_returns_value() {
    let mut input = TextInput::new();
    input.set_value("test");
    let msg = input.handle_key(&key(KeyCode::Enter));
    assert!(matches!(msg, Some(TextInputMsg::Submit(s)) if s == "test"));
}

#[test]
fn blur_ignores_input() {
    let mut input = TextInput::new();
    input.blur();
    input.handle_key(&key(KeyCode::Char('x')));
    assert_eq!(input.value(), "");
}

#[test]
fn delete_key() {
    let mut input = TextInput::new();
    input.set_value("abc");
    input.handle_key(&key(KeyCode::Home));
    input.handle_key(&key(KeyCode::Delete));
    assert_eq!(input.value(), "bc");
}

#[test]
fn mask_mode() {
    let input = TextInput::new().with_mask('*');
    assert_eq!(input.mask_char, Some('*'));

    let mut masked = TextInput::new().with_mask('*');
    masked.set_value("你好");
    masked.blur();
    assert_eq!(masked.view(), "**");
}

#[test]
fn prefix_in_view() {
    let mut input = TextInput::new().with_prefix("> ");
    input.set_value("hello");
    input.blur();
    let view = input.view();
    assert!(view.starts_with("> "));
}
