use a3s_tui::{
    Event, InputCapture, InputCaptureMode, InputRoute, InputRouter, InputScope, KeyBinding,
    KeyCode, KeyEvent, KeyModifiers, RoutedInput,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Close,
    Down,
    Help,
    Quit,
    Submit,
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn focused_bindings_take_priority_over_global_bindings() {
    let router = InputRouter::new()
        .bind_global(KeyBinding::new(KeyCode::Enter), Action::Submit, "Submit")
        .bind_focus(
            7,
            KeyBinding::new(KeyCode::Enter),
            Action::Down,
            "Open item",
        );

    let routed = router.resolve_key(&key(KeyCode::Enter), Some(7));

    assert_eq!(
        routed,
        Some(RoutedInput {
            action: Action::Down,
            route: InputRoute::Focus(7),
        })
    );
}

#[test]
fn global_bindings_resolve_without_focus() {
    let router =
        InputRouter::new().bind_global(KeyBinding::ctrl(KeyCode::Char('c')), Action::Quit, "Quit");

    let routed = router.resolve_key(
        &KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        },
        None,
    );

    assert_eq!(
        routed,
        Some(RoutedInput {
            action: Action::Quit,
            route: InputRoute::Global,
        })
    );
}

#[test]
fn exclusive_capture_handles_own_keys_and_blocks_background() {
    let mut router = InputRouter::new()
        .bind_global(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit")
        .bind_focus(1, KeyBinding::new(KeyCode::Enter), Action::Submit, "Submit")
        .bind_scope(
            "palette",
            KeyBinding::new(KeyCode::Esc),
            Action::Close,
            "Close",
        );

    router.push_capture("palette");

    assert_eq!(
        router.resolve_key(&key(KeyCode::Esc), Some(1)),
        Some(RoutedInput {
            action: Action::Close,
            route: InputRoute::Captured(InputScope::named("palette")),
        })
    );
    assert_eq!(router.resolve_key(&key(KeyCode::Enter), Some(1)), None);
    assert_eq!(router.resolve_key(&key(KeyCode::Char('q')), Some(1)), None);
}

#[test]
fn passthrough_capture_allows_background_when_unhandled() {
    let mut router = InputRouter::new()
        .bind_global(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit")
        .bind_scope(
            "hints",
            KeyBinding::new(KeyCode::Char('?')),
            Action::Help,
            "Help",
        );

    router.push_capture_with_mode("hints", InputCaptureMode::Passthrough);

    assert_eq!(
        router.resolve_key(&key(KeyCode::Char('?')), None),
        Some(RoutedInput {
            action: Action::Help,
            route: InputRoute::Captured(InputScope::named("hints")),
        })
    );
    assert_eq!(
        router.resolve_key(&key(KeyCode::Char('q')), None),
        Some(RoutedInput {
            action: Action::Quit,
            route: InputRoute::Global,
        })
    );
}

#[test]
fn captures_are_checked_newest_first() {
    let mut router = InputRouter::new()
        .bind_scope(
            "outer",
            KeyBinding::new(KeyCode::Esc),
            Action::Close,
            "Close outer",
        )
        .bind_scope(
            "inner",
            KeyBinding::new(KeyCode::Esc),
            Action::Quit,
            "Close inner",
        );

    router.push_capture("outer");
    router.push_capture("inner");

    assert_eq!(
        router.resolve_key(&key(KeyCode::Esc), None),
        Some(RoutedInput {
            action: Action::Quit,
            route: InputRoute::Captured(InputScope::named("inner")),
        })
    );
}

#[test]
fn remove_capture_removes_newest_matching_scope() {
    let mut router = InputRouter::<Action>::new();
    router.push_capture("palette");
    router.push_capture("help");
    router.push_capture("palette");

    assert!(router.remove_capture("palette"));
    assert_eq!(
        router.active_capture().map(InputCapture::scope),
        Some(&InputScope::named("help"))
    );
}

#[test]
fn resolve_event_ignores_non_key_events() {
    let router =
        InputRouter::new().bind_global(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit");

    assert_eq!(
        router.resolve_event(
            &Event::Resize {
                width: 80,
                height: 24
            },
            None
        ),
        None
    );
}

#[test]
fn active_help_matches_exclusive_capture_boundary() {
    let mut router = InputRouter::new()
        .bind_global(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit")
        .bind_focus(2, KeyBinding::new(KeyCode::Enter), Action::Submit, "Submit")
        .bind_scope(
            "palette",
            KeyBinding::new(KeyCode::Esc),
            Action::Close,
            "Close",
        );

    router.push_capture("palette");

    let help = router.active_help(Some(2));

    assert_eq!(help.len(), 1);
    assert_eq!(
        help[0].route,
        InputRoute::Captured(InputScope::named("palette"))
    );
    assert_eq!(help[0].description, "Close");
}

#[test]
fn active_help_includes_passthrough_capture_focus_and_global() {
    let mut router = InputRouter::new()
        .bind_global(KeyBinding::new(KeyCode::Char('q')), Action::Quit, "Quit")
        .bind_focus(2, KeyBinding::new(KeyCode::Enter), Action::Submit, "Submit")
        .bind_scope(
            "hints",
            KeyBinding::new(KeyCode::Char('?')),
            Action::Help,
            "Help",
        );

    router.push_capture_with_mode("hints", InputCaptureMode::Passthrough);

    let help = router.active_help(Some(2));

    assert_eq!(help.len(), 3);
    assert!(help
        .iter()
        .any(|entry| entry.route == InputRoute::Captured(InputScope::named("hints"))));
    assert!(help.iter().any(|entry| entry.route == InputRoute::Focus(2)));
    assert!(help.iter().any(|entry| entry.route == InputRoute::Global));
}
