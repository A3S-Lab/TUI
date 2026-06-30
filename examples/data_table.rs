use a3s_tui::cmd;
use a3s_tui::components::{CellAlign, DataColumn, DataRow, DataTable, StatusBar};
use a3s_tui::event::KeyEvent;
use a3s_tui::layout::{Constraint, Layout};
use a3s_tui::style::{Color, Style};
use a3s_tui::{Event, KeyCode, KeyModifiers, Model, ProgramBuilder};

struct App {
    selected: usize,
    scroll: usize,
    width: u16,
    height: u16,
}

enum Msg {
    Event(Event),
    Quit,
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

impl Model for App {
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
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.selected = self.selected.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.selected = (self.selected + 1).min(rows().len().saturating_sub(1));
                    }
                    _ => {}
                }
                let body = (self.height as usize).saturating_sub(4).max(1);
                if self.selected < self.scroll {
                    self.scroll = self.selected;
                } else if self.selected >= self.scroll + body {
                    self.scroll = self.selected + 1 - body;
                }
                None
            }
            Msg::Event(_) => None,
        }
    }

    fn view(&self) -> String {
        let header = Style::new()
            .fg(Color::BrightWhite)
            .bg(Color::Rgb(35, 40, 60))
            .bold()
            .width(self.width)
            .render(" a3s-tui DataTable demo");
        let table = DataTable::new(vec![
            DataColumn::new("PID").width(7).align(CellAlign::Right),
            DataColumn::new("AGENT").width(10),
            DataColumn::new("CPU%").width(6).align(CellAlign::Right),
            DataColumn::new("MEM%").width(6).align(CellAlign::Right),
            DataColumn::new("RISK").width(5),
            DataColumn::new("COMMAND").min_width(12),
        ])
        .selected(Some(self.selected))
        .scroll(self.scroll)
        .empty("no rows")
        .row(
            DataRow::new(vec![
                "1204",
                "codex",
                "23.1",
                "3.4",
                "med",
                "codex exec run the verification suite",
            ])
            .fg(Color::Rgb(16, 163, 127)),
        )
        .row(
            DataRow::new(vec![
                "931",
                "claude",
                "4.0",
                "2.1",
                "low",
                "node /usr/local/bin/claude --continue",
            ])
            .fg(Color::Rgb(217, 119, 87)),
        )
        .row(
            DataRow::new(vec![
                "4482",
                "a3s-code",
                "1.2",
                "1.9",
                "low",
                "a3s code resume 18bc520834895428-1265e",
            ])
            .fg(Color::Rgb(122, 162, 247)),
        )
        .view(self.width, (self.height as usize).saturating_sub(2));
        let status = StatusBar::new()
            .left(" ↑/↓ select")
            .center("DataTable: width fitting · selection · row colors")
            .right("q quit ")
            .fg(Color::BrightWhite)
            .bg(Color::Rgb(35, 40, 60))
            .view(self.width);

        Layout::vertical()
            .item(&header, Constraint::Fixed(1))
            .item(&table, Constraint::Fill)
            .item(&status, Constraint::Fixed(1))
            .render(self.height)
    }
}

fn rows() -> [&'static str; 3] {
    ["codex", "claude", "a3s-code"]
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (width, height) = a3s_tui::terminal::Terminal::size().unwrap_or((100, 24));
    ProgramBuilder::new(App {
        selected: 0,
        scroll: 0,
        width,
        height,
    })
    .with_alt_screen()
    .with_fps(30)
    .run()
    .await
}
