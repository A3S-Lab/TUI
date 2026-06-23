use a3s_tui::cmd;
use a3s_tui::element::TextElement;
use a3s_tui::event::KeyEvent;
use a3s_tui::style::Color;
use a3s_tui::{
    col, text, Element, ElementModel, ElementProgramBuilder, Event, KeyCode, KeyModifiers,
};

struct Counter {
    count: i64,
}

enum Msg {
    Increment,
    Decrement,
    Reset,
    Quit,
    Noop,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) => Msg::Quit,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
            }) if modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
            Event::Key(KeyEvent {
                code: KeyCode::Up, ..
            }) => Msg::Increment,
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                ..
            }) => Msg::Decrement,
            Event::Key(KeyEvent {
                code: KeyCode::Char('r'),
                ..
            }) => Msg::Reset,
            _ => Msg::Noop,
        }
    }
}

impl ElementModel for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Increment => {
                self.count += 1;
                None
            }
            Msg::Decrement => {
                self.count -= 1;
                None
            }
            Msg::Reset => {
                self.count = 0;
                None
            }
            Msg::Quit => Some(cmd::quit()),
            Msg::Noop => None,
        }
    }

    fn view(&self) -> Element<Msg> {
        col![
            text!(""),
            Element::Text(
                TextElement::new(format!("  Counter: {}", self.count))
                    .bold()
                    .fg(Color::Cyan)
            ),
            text!(""),
            Element::Text(TextElement::new("  Up/Down to change | r to reset | q to quit").dim()),
        ]
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let model = Counter { count: 0 };

    ElementProgramBuilder::new(model)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
