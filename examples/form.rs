use a3s_tui::cmd;
use a3s_tui::components::{Alert, AlertKind, TextInput};
use a3s_tui::element::{BoxElement, FlexDirection, TextElement};
use a3s_tui::style::Color;
use a3s_tui::{col, Element, ElementModel, ElementProgramBuilder, Event, KeyCode};
use std::time::Duration;

struct FormApp {
    name: TextInput,
    email: TextInput,
    password: TextInput,
    focused_field: usize,
    submitted: bool,
    error: Option<String>,
}

enum Msg {
    Quit,
    NextField,
    PrevField,
    Submit,
    KeyPress(Event),
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(k) if k.is_ctrl('q') => Msg::Quit,
            Event::Key(k) if k.is_key(KeyCode::Tab) => Msg::NextField,
            Event::Key(k) if k.is_key(KeyCode::BackTab) => Msg::PrevField,
            Event::Key(k) if k.is_ctrl('s') => Msg::Submit,
            _ => Msg::KeyPress(event),
        }
    }
}

impl ElementModel for FormApp {
    type Msg = Msg;

    fn init(&mut self) -> Option<cmd::Cmd<Msg>> {
        self.name.focus();
        None
    }

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Quit => Some(cmd::quit()),
            Msg::NextField => {
                self.blur_all();
                self.focused_field = (self.focused_field + 1) % 3;
                self.focus_current();
                None
            }
            Msg::PrevField => {
                self.blur_all();
                self.focused_field = if self.focused_field == 0 { 2 } else { self.focused_field - 1 };
                self.focus_current();
                None
            }
            Msg::Submit => {
                if self.validate() {
                    self.submitted = true;
                    Some(cmd::delay(Duration::from_secs(2), Msg::Quit))
                } else {
                    None
                }
            }
            Msg::KeyPress(event) => {
                if let Event::Key(key_event) = event {
                    match self.focused_field {
                        0 => { self.name.handle_key(&key_event); }
                        1 => { self.email.handle_key(&key_event); }
                        2 => { self.password.handle_key(&key_event); }
                        _ => {}
                    }
                }
                None
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let title = Element::Text(
            TextElement::new("User Registration Form")
                .bold()
                .fg(Color::Cyan)
        );

        let name_label = Element::Text(TextElement::new("Name:").fg(Color::BrightBlack));
        let name_input = self.name.element();

        let email_label = Element::Text(TextElement::new("Email:").fg(Color::BrightBlack));
        let email_input = self.email.element();

        let password_label = Element::Text(TextElement::new("Password:").fg(Color::BrightBlack));
        let password_input = self.password.element();

        let help = Element::Text(
            TextElement::new("Tab: next | Shift+Tab: prev | Ctrl+S: submit | Ctrl+Q: quit")
                .fg(Color::gray(150))
        );

        let mut content = col![
            Element::Text(TextElement::new("")),
            title,
            Element::Text(TextElement::new("")),
            name_label,
            name_input,
            Element::Text(TextElement::new("")),
            email_label,
            email_input,
            Element::Text(TextElement::new("")),
            password_label,
            password_input,
            Element::Text(TextElement::new("")),
            help,
        ];

        if let Some(ref err) = self.error {
            content = col![
                content,
                Element::Text(TextElement::new("")),
                Alert::new(AlertKind::Error, err).element(),
            ];
        }

        if self.submitted {
            content = col![
                content,
                Element::Text(TextElement::new("")),
                Alert::new(AlertKind::Success, "Registration successful!")
                    .title("Success")
                    .element(),
            ];
        }

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .padding(2)
                .child(content)
        )
    }
}

impl FormApp {
    fn blur_all(&mut self) {
        self.name.blur();
        self.email.blur();
        self.password.blur();
    }

    fn focus_current(&mut self) {
        match self.focused_field {
            0 => self.name.focus(),
            1 => self.email.focus(),
            2 => self.password.focus(),
            _ => {}
        }
    }

    fn validate(&mut self) -> bool {
        self.error = None;

        if self.name.value().trim().is_empty() {
            self.error = Some("Name is required".to_string());
            return false;
        }

        if self.email.value().trim().is_empty() {
            self.error = Some("Email is required".to_string());
            return false;
        }

        if !self.email.value().contains('@') {
            self.error = Some("Invalid email format".to_string());
            return false;
        }

        if self.password.value().len() < 6 {
            self.error = Some("Password must be at least 6 characters".to_string());
            return false;
        }

        true
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = FormApp {
        name: TextInput::new().with_prefix("> "),
        email: TextInput::new().with_prefix("> "),
        password: TextInput::new().with_prefix("> ").with_mask('*'),
        focused_field: 0,
        submitted: false,
        error: None,
    };

    ElementProgramBuilder::new(app)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
