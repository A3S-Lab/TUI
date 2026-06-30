use a3s_tui::cmd;
use a3s_tui::components::{Progress, Spinner, StatusBar, Table};
use a3s_tui::element::{BoxElement, FlexDirection, TextElement};
use a3s_tui::style::Color;
use a3s_tui::{col, row, Element, ElementModel, ElementProgramBuilder, Event};
use std::time::Duration;

struct Dashboard {
    cpu_usage: f64,
    memory_usage: f64,
    disk_usage: f64,
    spinner: Spinner,
    uptime_seconds: u64,
    requests_per_sec: u32,
}

enum Msg {
    Quit,
    Tick,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(k) if k.is_char('q') => Msg::Quit,
            Event::Key(k) if k.is_ctrl('c') => Msg::Quit,
            _ => Msg::Tick,
        }
    }
}

impl ElementModel for Dashboard {
    type Msg = Msg;

    fn init(&mut self) -> Option<cmd::Cmd<Msg>> {
        Some(cmd::tick(Duration::from_millis(100), Msg::Tick))
    }

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Quit => Some(cmd::quit()),
            Msg::Tick => {
                self.uptime_seconds += 1;
                self.spinner.tick();

                // Simulate metrics fluctuation
                self.cpu_usage = (self.cpu_usage + 0.02).min(0.95);
                self.memory_usage = 0.65 + (self.uptime_seconds as f64 * 0.001).sin() * 0.1;
                self.disk_usage = 0.42;
                self.requests_per_sec =
                    150 + ((self.uptime_seconds as f64 * 0.5).sin() * 50.0) as u32;

                Some(cmd::tick(Duration::from_millis(100), Msg::Tick))
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let title = Element::Text(
            TextElement::new("System Dashboard")
                .bold()
                .fg(Color::BrightCyan),
        );

        // Metrics section
        let cpu_label = Element::Text(TextElement::new("CPU Usage:").fg(Color::BrightBlack));
        let cpu_progress = Progress::new()
            .value(self.cpu_usage)
            .filled_color(self.get_usage_color(self.cpu_usage))
            .element();

        let memory_label = Element::Text(TextElement::new("Memory:").fg(Color::BrightBlack));
        let memory_progress = Progress::new()
            .value(self.memory_usage)
            .filled_color(self.get_usage_color(self.memory_usage))
            .element();

        let disk_label = Element::Text(TextElement::new("Disk:").fg(Color::BrightBlack));
        let disk_progress = Progress::new()
            .value(self.disk_usage)
            .filled_color(self.get_usage_color(self.disk_usage))
            .element();

        let metrics = col![
            Element::Text(TextElement::new("System Metrics").bold().fg(Color::White)),
            Element::Text(TextElement::new("")),
            cpu_label,
            cpu_progress,
            Element::Text(TextElement::new("")),
            memory_label,
            memory_progress,
            Element::Text(TextElement::new("")),
            disk_label,
            disk_progress,
        ];

        // Stats table
        let stats_table = Table::new(vec!["Metric", "Value"])
            .row(vec!["Uptime", &format!("{}s", self.uptime_seconds)])
            .row(vec!["Requests/sec", &format!("{}", self.requests_per_sec)])
            .row(vec!["Active Connections", "42"])
            .row(vec!["Error Rate", "0.02%"])
            .element();

        let stats = col![
            Element::Text(TextElement::new("Statistics").bold().fg(Color::White)),
            Element::Text(TextElement::new("")),
            stats_table,
        ];

        // Services status
        let services_table = Table::new(vec!["Service", "Status", "Uptime"])
            .row(vec!["API Server", "Running", "99.9%"])
            .row(vec!["Database", "Running", "99.8%"])
            .row(vec!["Cache", "Running", "100%"])
            .row(vec!["Queue", "Running", "99.7%"])
            .element();

        let services = col![
            Element::Text(TextElement::new("Services").bold().fg(Color::White)),
            Element::Text(TextElement::new("")),
            services_table,
        ];

        // Loading indicator
        let loading = row![
            Element::Text(TextElement::new("Refreshing data ")),
            Element::Text(TextElement::new(self.spinner.view()).fg(Color::Cyan)),
        ];

        // Status bar
        let status_bar = StatusBar::new()
            .left(format!("Uptime: {}s", self.uptime_seconds))
            .right("Press 'q' to quit")
            .fg(Color::BrightBlack)
            .bg(Color::gray(20))
            .element();

        // Layout
        let content = col![
            Element::Text(TextElement::new("")),
            title,
            Element::Text(TextElement::new("")),
            row![
                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Column)
                        .child(metrics)
                ),
                Element::Text(TextElement::new("    ")),
                Element::Box(
                    BoxElement::new()
                        .direction(FlexDirection::Column)
                        .child(stats)
                ),
            ],
            Element::Text(TextElement::new("")),
            services,
            Element::Text(TextElement::new("")),
            loading,
            Element::Text(TextElement::new("")),
            status_bar,
        ];

        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .padding(2)
                .child(content),
        )
    }
}

impl Dashboard {
    fn get_usage_color(&self, usage: f64) -> Color {
        if usage > 0.8 {
            Color::Red
        } else if usage > 0.6 {
            Color::Yellow
        } else {
            Color::Green
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = Dashboard {
        cpu_usage: 0.45,
        memory_usage: 0.65,
        disk_usage: 0.42,
        spinner: Spinner::new(),
        uptime_seconds: 0,
        requests_per_sec: 150,
    };

    ElementProgramBuilder::new(app)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
