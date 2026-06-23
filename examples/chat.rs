use a3s_tui::cmd::{self, Cmd};
use a3s_tui::components::modal::{Modal, ModalMsg};
use a3s_tui::components::textarea::TextareaMsg;
use a3s_tui::components::{Spinner, StatusBar, Textarea, Viewport};
use a3s_tui::event::KeyEvent;
use a3s_tui::keymap::{KeyBinding, Keymap};
use a3s_tui::layout::{Constraint, Layout};
use a3s_tui::streaming::StreamingMarkdown;
use a3s_tui::style::{Color, Style};
use a3s_tui::{Event, KeyCode, KeyModifiers, Model, ProgramBuilder};
use std::time::Duration;

const RESPONSES: &[&str] = &[
    "# Sure!\n\nHere's a quick **Rust** example:\n\n```rust\nfn fibonacci(n: u64) -> u64 {\n    match n {\n        0 => 0,\n        1 => 1,\n        _ => fibonacci(n - 1) + fibonacci(n - 2),\n    }\n}\n```\n\nThis uses pattern matching for the base cases. For production use, consider memoization or an iterative approach.\n",
    "# File Edit\n\nI'll update `src/main.rs` with the new handler:\n\n```rust\nasync fn handle_request(req: Request) -> Response {\n    let body = req.body().await?;\n    let parsed: MyStruct = serde_json::from_slice(&body)?;\n    Ok(Response::json(&parsed))\n}\n```\n\n> Note: This requires the `serde_json` crate.\n\nShall I proceed with this change?\n",
    "Done! I've made the following changes:\n\n- Added error handling to the parser\n- Updated the test suite\n- Fixed the CI configuration\n\nAll tests pass. Let me know if you need anything else.\n",
];

struct App {
    viewport: Viewport,
    textarea: Textarea,
    spinner: Spinner,
    streaming: StreamingMarkdown,
    state: AppState,
    stream_pos: usize,
    current_response: usize,
    messages: Vec<String>,
    modal: Option<Modal>,
    width: u16,
    height: u16,
    keymap: Keymap<Action>,
}

#[derive(Clone)]
#[allow(dead_code)]
enum Action {
    Quit,
    ScrollUp,
    ScrollDown,
    ScrollTop,
    ScrollBottom,
    ShowHelp,
}

#[derive(PartialEq)]
enum AppState {
    Idle,
    Streaming,
    ModalOpen,
}

enum Msg {
    Event(Event),
    Quit,
    Submit(String),
    StreamToken,
    SpinnerTick,
    ModalConfirm(usize),
    ModalDismiss,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
            }) if modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
            _ => Msg::Event(event),
        }
    }
}

impl Model for App {
    type Msg = Msg;

    fn init(&mut self) -> Option<Cmd<Msg>> {
        let welcome = Style::new()
            .fg(Color::BrightBlack)
            .italic()
            .render("  Welcome to a3s-tui chat. Type a message and press Enter.\n  Ctrl+C to quit | PageUp/PageDown to scroll\n");
        self.viewport.set_content(&welcome);
        None
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd<Msg>> {
        match msg {
            Msg::Quit => return Some(cmd::quit()),

            Msg::Event(Event::Resize { width, height }) => {
                self.width = width;
                self.height = height;
                self.viewport.resize(width, height.saturating_sub(7));
                self.streaming = StreamingMarkdown::new(width as usize - 2);
            }

            Msg::Event(Event::Key(key)) => {
                if self.state == AppState::ModalOpen {
                    return self.handle_modal_key(&key);
                }

                if let Some(action) = self.keymap.resolve(&key) {
                    match action {
                        Action::Quit => return Some(cmd::quit()),
                        Action::ScrollUp => {
                            self.viewport
                                .update(a3s_tui::components::viewport::ViewportMsg::PageUp);
                        }
                        Action::ScrollDown => {
                            self.viewport
                                .update(a3s_tui::components::viewport::ViewportMsg::PageDown);
                        }
                        Action::ScrollTop => {
                            self.viewport
                                .update(a3s_tui::components::viewport::ViewportMsg::Top);
                        }
                        Action::ScrollBottom => {
                            self.viewport
                                .update(a3s_tui::components::viewport::ViewportMsg::Bottom);
                        }
                        Action::ShowHelp => {}
                    }
                    return None;
                }

                if self.state == AppState::Streaming {
                    return None;
                }

                if let Some(TextareaMsg::Submit(text)) = self.textarea.handle_key(&key) {
                    return Some(cmd::msg(Msg::Submit(text)));
                }
            }

            Msg::Submit(text) => {
                if text.trim().is_empty() {
                    return None;
                }
                let user_msg = Style::new()
                    .bold()
                    .fg(Color::BrightGreen)
                    .render(&format!("> {}", text));
                self.messages.push(user_msg.clone());

                let mut full_content = self.messages.join("\n\n");
                full_content.push_str("\n\n");
                self.viewport.set_content(&full_content);
                self.textarea.clear();

                if text.to_lowercase().contains("edit") || text.to_lowercase().contains("write") {
                    self.state = AppState::ModalOpen;
                    self.modal = Some(
                        Modal::new()
                            .title("Permission Required")
                            .body("Allow file write to src/main.rs?")
                            .options(vec!["Allow", "Deny", "Always Allow"]),
                    );
                    return None;
                }

                self.start_streaming();
                return Some(cmd::batch(vec![
                    cmd::tick(Duration::from_millis(20), Msg::StreamToken),
                    cmd::tick(Duration::from_millis(80), Msg::SpinnerTick),
                ]));
            }

            Msg::StreamToken => {
                let response = RESPONSES[self.current_response % RESPONSES.len()];
                if self.stream_pos < response.len() {
                    let chunk_end = (self.stream_pos + 4).min(response.len());
                    let chunk = &response[self.stream_pos..chunk_end];
                    self.streaming.push(chunk);
                    self.stream_pos = chunk_end;

                    self.update_viewport_with_stream();
                    return Some(cmd::tick(Duration::from_millis(20), Msg::StreamToken));
                } else {
                    self.finish_streaming();
                }
            }

            Msg::SpinnerTick => {
                self.spinner.tick();
                if self.state == AppState::Streaming {
                    return Some(cmd::tick(Duration::from_millis(80), Msg::SpinnerTick));
                }
            }

            Msg::ModalConfirm(idx) => {
                self.modal = None;
                let choice = match idx {
                    0 => "Allowed",
                    1 => "Denied",
                    _ => "Always Allowed",
                };
                let notice = Style::new()
                    .fg(Color::Yellow)
                    .render(&format!("  [{}]", choice));
                self.messages.push(notice);
                self.state = AppState::Idle;

                if idx != 1 {
                    self.start_streaming();
                    return Some(cmd::batch(vec![
                        cmd::tick(Duration::from_millis(20), Msg::StreamToken),
                        cmd::tick(Duration::from_millis(80), Msg::SpinnerTick),
                    ]));
                } else {
                    let denied = Style::new()
                        .fg(Color::Red)
                        .render("  Operation denied by user.");
                    self.messages.push(denied);
                    self.rebuild_viewport();
                }
            }

            Msg::ModalDismiss => {
                self.modal = None;
                self.state = AppState::Idle;
            }

            _ => {}
        }
        None
    }

    fn view(&self) -> String {
        if self.state == AppState::ModalOpen {
            if let Some(ref modal) = self.modal {
                return modal.view(self.width, self.height);
            }
        }

        let status_text = match self.state {
            AppState::Streaming => format!(" {} Generating...", self.spinner.view()),
            AppState::Idle => " a3s-tui chat".to_string(),
            AppState::ModalOpen => " Waiting for permission...".to_string(),
        };

        let status = StatusBar::new()
            .left(&status_text)
            .right("Ctrl+C quit | PgUp/PgDn scroll ")
            .fg(Color::White)
            .bg(Color::BrightBlack)
            .view(self.width);

        let viewport_view = self.viewport.view();

        let separator = Style::new()
            .fg(Color::BrightBlack)
            .render(&"─".repeat(self.width as usize));

        let prompt = Style::new().fg(Color::BrightGreen).bold().render("❯ ");
        let input_view = format!("{}{}", prompt, self.textarea.view());

        Layout::vertical()
            .item(&status, Constraint::Fixed(1))
            .item(&viewport_view, Constraint::Fill)
            .item(&separator, Constraint::Fixed(1))
            .item(&input_view, Constraint::Fixed(3))
            .render(self.height)
    }
}

impl App {
    fn start_streaming(&mut self) {
        self.state = AppState::Streaming;
        self.stream_pos = 0;
        self.streaming.clear();
        self.spinner.start();
        self.current_response += 1;
    }

    fn finish_streaming(&mut self) {
        self.state = AppState::Idle;
        self.spinner.stop();
        let rendered = self.streaming.view();
        self.messages.push(rendered);
        self.rebuild_viewport();
    }

    fn update_viewport_with_stream(&mut self) {
        let rendered = self.streaming.view();
        let mut full = self.messages.join("\n\n");
        if !full.is_empty() {
            full.push_str("\n\n");
        }
        full.push_str(&rendered);
        self.viewport.set_content(&full);
    }

    fn rebuild_viewport(&mut self) {
        let full = self.messages.join("\n\n");
        self.viewport.set_content(&format!("{}\n", full));
    }

    fn handle_modal_key(&mut self, key: &KeyEvent) -> Option<Cmd<Msg>> {
        if let Some(ref mut modal) = self.modal {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    modal.update(ModalMsg::Prev);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    modal.update(ModalMsg::Next);
                }
                KeyCode::Enter => {
                    let idx = modal.confirm();
                    return Some(cmd::msg(Msg::ModalConfirm(idx)));
                }
                KeyCode::Esc => {
                    return Some(cmd::msg(Msg::ModalDismiss));
                }
                _ => {}
            }
        }
        None
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (width, height) = a3s_tui::terminal::Terminal::size().unwrap_or((80, 24));

    let keymap = Keymap::new()
        .bind(
            KeyBinding::new(KeyCode::PageUp),
            Action::ScrollUp,
            "Scroll up",
        )
        .bind(
            KeyBinding::new(KeyCode::PageDown),
            Action::ScrollDown,
            "Scroll down",
        )
        .bind(
            KeyBinding::ctrl(KeyCode::Home),
            Action::ScrollTop,
            "Scroll to top",
        )
        .bind(
            KeyBinding::ctrl(KeyCode::End),
            Action::ScrollBottom,
            "Scroll to bottom",
        );

    let app = App {
        viewport: Viewport::new(width, height.saturating_sub(7)),
        textarea: Textarea::new()
            .with_height(3)
            .with_width(width)
            .with_submit_on_enter(true),
        spinner: Spinner::new().with_title(""),
        streaming: StreamingMarkdown::new(width as usize - 2),
        state: AppState::Idle,
        stream_pos: 0,
        current_response: 0,
        messages: Vec::new(),
        modal: None,
        width,
        height,
        keymap,
    };

    ProgramBuilder::new(app)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
