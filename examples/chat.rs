use a3s_tui::cmd::{self, Cmd};
use a3s_tui::components::{Spinner, StatusBar, Textarea, Viewport};
use a3s_tui::event::KeyEvent;
use a3s_tui::layout::{Constraint, Layout};
use a3s_tui::streaming::StreamingMarkdown;
use a3s_tui::style::{Color, Style};
use a3s_tui::{Event, KeyCode, KeyModifiers, Model, ProgramBuilder};
use std::time::Duration;

const SAMPLE_RESPONSE: &str = r#"# Hello!

I can help you with that. Here's a quick example:

```rust
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

This function takes a `name` parameter and returns a greeting string.

- It uses Rust's `format!` macro
- The return type is `String`
- No explicit `return` needed

Let me know if you need anything else!
"#;

struct App {
    viewport: Viewport,
    textarea: Textarea,
    spinner: Spinner,
    streaming: StreamingMarkdown,
    state: AppState,
    stream_pos: usize,
    width: u16,
    height: u16,
}

#[derive(PartialEq)]
enum AppState {
    Idle,
    Streaming,
}

enum Msg {
    Event(Event),
    Quit,
    Submit(String),
    StreamToken,
    SpinnerTick,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers })
                if modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
            Event::Key(KeyEvent { code: KeyCode::Char('q'), .. })
                if false => Msg::Quit, // disabled during normal use
            _ => Msg::Event(event),
        }
    }
}

impl Model for App {
    type Msg = Msg;

    fn init(&mut self) -> Option<Cmd<Msg>> {
        None
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd<Msg>> {
        match msg {
            Msg::Quit => return Some(cmd::quit()),
            Msg::Event(Event::Resize { width, height }) => {
                self.width = width;
                self.height = height;
                self.viewport.resize(width, height.saturating_sub(8));
            }
            Msg::Event(Event::Key(key)) => {
                if self.state == AppState::Streaming {
                    return None;
                }
                if let Some(a3s_tui::components::textarea::TextareaMsg::Submit(text)) =
                    self.textarea.handle_key(&key)
                {
                    return Some(cmd::msg(Msg::Submit(text)));
                }
            }
            Msg::Submit(text) => {
                let user_msg = Style::new()
                    .bold()
                    .fg(Color::BrightGreen)
                    .render(&format!("> {}", text));
                self.viewport.append(&format!("{}\n\n", user_msg));
                self.textarea.clear();
                self.state = AppState::Streaming;
                self.stream_pos = 0;
                self.streaming.clear();
                self.spinner.start();
                return Some(cmd::batch(vec![
                    cmd::tick(Duration::from_millis(30), Msg::StreamToken),
                    cmd::tick(Duration::from_millis(80), Msg::SpinnerTick),
                ]));
            }
            Msg::StreamToken => {
                if self.stream_pos < SAMPLE_RESPONSE.len() {
                    let chunk_end = (self.stream_pos + 3).min(SAMPLE_RESPONSE.len());
                    let chunk = &SAMPLE_RESPONSE[self.stream_pos..chunk_end];
                    self.streaming.push(chunk);
                    self.stream_pos = chunk_end;

                    let rendered = self.streaming.view();
                    self.viewport.set_content(&rendered);

                    return Some(cmd::tick(Duration::from_millis(30), Msg::StreamToken));
                } else {
                    self.state = AppState::Idle;
                    self.spinner.stop();
                    self.viewport.append("\n");
                }
            }
            Msg::SpinnerTick => {
                self.spinner.tick();
                if self.state == AppState::Streaming {
                    return Some(cmd::tick(Duration::from_millis(80), Msg::SpinnerTick));
                }
            }
            _ => {}
        }
        None
    }

    fn view(&self) -> String {
        let status = StatusBar::new()
            .left(if self.state == AppState::Streaming {
                format!(" {} Thinking...", self.spinner.view())
            } else {
                " a3s-tui chat".to_string()
            })
            .right("Ctrl+C to quit ")
            .fg(Color::White)
            .bg(Color::BrightBlack)
            .view(self.width);

        let viewport_view = self.viewport.view();

        let separator = Style::new()
            .fg(Color::BrightBlack)
            .render(&"─".repeat(self.width as usize));

        let input_label = Style::new().fg(Color::BrightGreen).bold().render("> ");
        let input_view = format!("{}{}", input_label, self.textarea.view());

        Layout::vertical()
            .item(&status, Constraint::Fixed(1))
            .item(&viewport_view, Constraint::Fill)
            .item(&separator, Constraint::Fixed(1))
            .item(&input_view, Constraint::Fixed(3))
            .render(self.height)
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (width, height) = a3s_tui::terminal::Terminal::size().unwrap_or((80, 24));

    let app = App {
        viewport: Viewport::new(width, height.saturating_sub(8)),
        textarea: Textarea::new()
            .with_height(3)
            .with_width(width)
            .with_submit_on_enter(true),
        spinner: Spinner::new().with_title(""),
        streaming: StreamingMarkdown::new(width as usize),
        state: AppState::Idle,
        stream_pos: 0,
        width,
        height,
    };

    ProgramBuilder::new(app)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
