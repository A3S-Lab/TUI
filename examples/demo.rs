use a3s_tui::cmd;
use a3s_tui::components::{Alert, AlertKind, Badge, Select, Table, Tabs};
use a3s_tui::element::{BoxElement, FlexDirection, TextElement};
use a3s_tui::event::KeyEvent;
use a3s_tui::style::Color;
use a3s_tui::{
    col, row, Element, ElementModel, ElementProgramBuilder, Event, KeyCode, KeyModifiers,
};

struct Demo {
    tabs: Tabs,
    select: Select,
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
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) => Msg::Quit,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
            }) if modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
            _ => Msg::Event(event),
        }
    }
}

impl ElementModel for Demo {
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
                self.tabs.handle_key(&key);
                self.select.handle_key(&key);
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
                    TextElement::new(" a3s-tui Component Demo")
                        .bold()
                        .fg(Color::White),
                ))
                .child(Element::Spacer)
                .child(Element::Text(
                    TextElement::new("q to quit ").fg(Color::BrightWhite),
                )),
        );

        let tabs_view = self.tabs.element();

        let badge_section = row![
            Badge::new("INFO").color(Color::Blue).element(),
            Element::Text(TextElement::new(" ")),
            Badge::new("OK").color(Color::Green).element(),
            Element::Text(TextElement::new(" ")),
            Badge::new("WARN").color(Color::Yellow).element(),
            Element::Text(TextElement::new(" ")),
            Badge::new("ERR").color(Color::Red).element(),
        ];

        let alert_view = Alert::new(AlertKind::Success, "All systems operational.")
            .title("Status")
            .element();

        let table_view = Table::new(vec!["Name", "Version", "Status"])
            .row(vec!["a3s-tui", "0.1.0", "Active"])
            .row(vec!["taffy", "0.7.7", "Active"])
            .row(vec!["crossterm", "0.28.1", "Active"])
            .row(vec!["comrak", "0.36.0", "Active"])
            .element();

        let select_view = self.select.element();

        col![
            header,
            Element::Text(TextElement::new("")),
            tabs_view,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Badges:").bold()),
            badge_section,
            Element::Text(TextElement::new("")),
            alert_view,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Table:").bold()),
            table_view,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Select (j/k + Enter):").bold()),
            select_view,
        ]
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (width, height) = a3s_tui::terminal::Terminal::size().unwrap_or((80, 24));

    let demo = Demo {
        tabs: Tabs::new(vec!["Overview", "Components", "Settings"]),
        select: Select::new(vec!["Option A", "Option B", "Option C", "Option D"]),
        width,
        height,
    };

    ElementProgramBuilder::new(demo)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
