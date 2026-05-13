use a3s_tui::cmd;
use a3s_tui::components::{Select, Tabs};
use a3s_tui::element::{BoxElement, FlexDirection, TextElement};
use a3s_tui::event::KeyEvent;
use a3s_tui::style::Color;
use a3s_tui::{col, Element, ElementModel, ElementProgramBuilder, Event, KeyCode, KeyModifiers};

struct App {
    tabs: Tabs,
    select: Select,
    last_event: String,
    width: u16,
    height: u16,
}

enum Msg {
    Quit,
    Event(Event),
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) => Msg::Quit,
            Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers })
                if modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
            _ => Msg::Event(event),
        }
    }
}

impl ElementModel for App {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Quit => Some(cmd::quit()),
            Msg::Event(Event::Resize { width, height }) => {
                self.width = width;
                self.height = height;
                None
            }
            Msg::Event(Event::Key(key)) => {
                self.last_event = format!("Key: {:?} {:?}", key.code, key.modifiers);
                self.tabs.handle_key(&key);
                self.select.handle_key(&key);
                None
            }
            Msg::Event(Event::Mouse(ref mouse)) => {
                self.last_event = format!(
                    "Mouse: {:?} at ({}, {})",
                    mouse.kind, mouse.column, mouse.row
                );
                self.tabs.handle_mouse(mouse);
                self.select.handle_mouse(mouse);
                None
            }
            _ => None,
        }
    }

    fn view(&self) -> Element<Msg> {
        let header = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .bg(Color::BrightBlack)
                .child(Element::Text(
                    TextElement::new(" Mouse Demo").bold().fg(Color::White),
                ))
                .child(Element::Spacer)
                .child(Element::Text(
                    TextElement::new("q to quit ").fg(Color::BrightWhite),
                )),
        );

        let event_display = Element::Text(
            TextElement::new(format!("  Last event: {}", self.last_event))
                .fg(Color::Yellow),
        );

        let tabs_label = Element::Text(
            TextElement::new("  Click a tab:").bold(),
        );

        let select_label = Element::Text(
            TextElement::new("  Click an item (or use j/k):").bold(),
        );

        col![
            header,
            Element::Text(TextElement::new("")),
            event_display,
            Element::Text(TextElement::new("")),
            tabs_label,
            self.tabs.element(),
            Element::Text(TextElement::new("")),
            select_label,
            self.select.element(),
        ]
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (width, height) = a3s_tui::terminal::Terminal::size().unwrap_or((80, 24));

    let app = App {
        tabs: Tabs::new(vec!["Files", "Search", "Git", "Debug"]),
        select: Select::new(vec![
            "Option Alpha",
            "Option Beta",
            "Option Gamma",
            "Option Delta",
        ]),
        last_event: "none".to_string(),
        width,
        height,
    };

    ElementProgramBuilder::new(app)
        .with_alt_screen()
        .with_mouse_support()
        .with_fps(30)
        .run()
        .await
}
