use a3s_tui::animation::{self, Easing, FrameAnimation, Transition};
use a3s_tui::cmd;
use a3s_tui::components::Progress;
use a3s_tui::element::{BoxElement, FlexDirection, TextElement};
use a3s_tui::event::KeyEvent;
use a3s_tui::theme::Theme;
use a3s_tui::{col, row, Element, ElementModel, ElementProgramBuilder, Event, KeyCode, KeyModifiers};
use std::time::Duration;

struct App {
    theme: Theme,
    theme_idx: usize,
    spinner: FrameAnimation,
    progress: Transition,
    tick_count: u64,
}

enum Msg {
    Quit,
    Tick,
    NextTheme,
    Noop,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) => Msg::Quit,
            Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers })
                if modifiers.contains(KeyModifiers::CONTROL) => Msg::Quit,
            Event::Key(KeyEvent { code: KeyCode::Char('t'), .. }) => Msg::NextTheme,
            _ => Msg::Noop,
        }
    }
}

impl ElementModel for App {
    type Msg = Msg;

    fn init(&mut self) -> Option<cmd::Cmd<Msg>> {
        Some(cmd::tick(Duration::from_millis(50), Msg::Tick))
    }

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Quit => Some(cmd::quit()),
            Msg::Tick => {
                self.tick_count += 1;
                self.spinner.tick();
                self.progress.tick();

                if !self.progress.is_animating() && self.progress.value() >= 1.0 {
                    self.progress.animate_to(0.0, Duration::from_millis(800), Easing::EaseInOut);
                } else if !self.progress.is_animating() && self.progress.value() <= 0.0 {
                    self.progress.animate_to(1.0, Duration::from_millis(1500), Easing::EaseOut);
                }

                Some(cmd::tick(Duration::from_millis(50), Msg::Tick))
            }
            Msg::NextTheme => {
                self.theme_idx = (self.theme_idx + 1) % 4;
                self.theme = match self.theme_idx {
                    0 => Theme::dark(),
                    1 => Theme::catppuccin(),
                    2 => Theme::tokyo_night(),
                    _ => Theme::light(),
                };
                None
            }
            Msg::Noop => None,
        }
    }

    fn view(&self) -> Element<Msg> {
        let theme = &self.theme;
        let theme_name = match self.theme_idx {
            0 => "Dark",
            1 => "Catppuccin Mocha",
            2 => "Tokyo Night",
            _ => "Light",
        };

        let header = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .bg(theme.surface)
                .child(Element::Text(
                    TextElement::new(" Theme & Animation Demo").bold().fg(theme.fg),
                ))
                .child(Element::Spacer)
                .child(Element::Text(
                    TextElement::new("t: theme | q: quit ").fg(theme.muted),
                )),
        );

        let theme_info = Element::Text(
            TextElement::new(format!("  Theme: {}", theme_name)).fg(theme.primary),
        );

        let colors_row = row![
            Element::Text(TextElement::new("  ").fg(theme.fg)),
            Element::Text(TextElement::new("■ primary").fg(theme.primary)),
            Element::Text(TextElement::new("  ")),
            Element::Text(TextElement::new("■ success").fg(theme.success)),
            Element::Text(TextElement::new("  ")),
            Element::Text(TextElement::new("■ warning").fg(theme.warning)),
            Element::Text(TextElement::new("  ")),
            Element::Text(TextElement::new("■ error").fg(theme.error)),
            Element::Text(TextElement::new("  ")),
            Element::Text(TextElement::new("■ info").fg(theme.info)),
        ];

        let spinner_text = format!("  {} Loading...", self.spinner.frame());
        let spinner_el = Element::Text(TextElement::new(spinner_text).fg(theme.primary));

        let progress_val = self.progress.value();
        let progress_el = Progress::new()
            .value(progress_val)
            .width(50)
            .filled_color(theme.primary)
            .empty_color(theme.muted)
            .element();

        let progress_row = row![
            Element::Text(TextElement::new("  ")),
            progress_el,
        ];

        let bar_width = 30;
        let pos = (progress_val * bar_width as f64) as usize;
        let bounce_bar: String = (0..bar_width)
            .map(|i| if i == pos { '●' } else { '─' })
            .collect();
        let bounce_el = Element::Text(
            TextElement::new(format!("  {}", bounce_bar)).fg(theme.secondary),
        );

        col![
            header,
            Element::Text(TextElement::new("")),
            theme_info,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Colors:").bold().fg(theme.fg)),
            colors_row,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Animation:").bold().fg(theme.fg)),
            spinner_el,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Progress (eased):").bold().fg(theme.fg)),
            progress_row,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Bounce:").bold().fg(theme.fg)),
            bounce_el,
        ]
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = App {
        theme: Theme::dark(),
        theme_idx: 0,
        spinner: animation::spinners::dots(),
        progress: {
            let mut t = Transition::new(0.0);
            t.animate_to(1.0, Duration::from_millis(1500), Easing::EaseOut);
            t
        },
        tick_count: 0,
    };

    ElementProgramBuilder::new(app)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
