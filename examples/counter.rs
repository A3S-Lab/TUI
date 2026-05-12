use a3s_tui::{cmd, Event, KeyCode, KeyEvent, KeyModifiers, Model, ProgramBuilder};

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

impl Model for Counter {
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

    fn view(&self) -> String {
        format!(
            "\n  Counter: {}\n\n  Up/Down to change | r to reset | q to quit\n",
            self.count
        )
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let model = Counter { count: 0 };

    ProgramBuilder::new(model)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
