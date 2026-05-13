use a3s_tui::cmd;
use a3s_tui::components::{
    Breadcrumb, Confirm, KeyValue, Paragraph, Scrollbar,
    ToastKind, ToastManager, Tree, TreeNode,
};
use a3s_tui::element::{BoxElement, FlexDirection, TextElement};
use a3s_tui::style::Color;
use a3s_tui::{col, row, Element, ElementModel, ElementProgramBuilder, Event};
use std::time::Duration;

struct App {
    toasts: ToastManager,
    confirm: Option<Confirm>,
    tick_count: u64,
}

enum Msg {
    Quit,
    Tick,
    ShowConfirm,
    AddToast,
    Noop,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match &event {
            Event::Key(k) if k.is_char('q') => Msg::Quit,
            Event::Key(k) if k.is_ctrl('c') => Msg::Quit,
            Event::Key(k) if k.is_char('c') => Msg::ShowConfirm,
            Event::Key(k) if k.is_char('t') => Msg::AddToast,
            _ => Msg::Noop,
        }
    }
}

impl ElementModel for App {
    type Msg = Msg;

    fn init(&mut self) -> Option<cmd::Cmd<Msg>> {
        Some(cmd::tick(Duration::from_millis(100), Msg::Tick))
    }

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Quit => Some(cmd::quit()),
            Msg::Tick => {
                self.tick_count += 1;
                self.toasts.tick();
                Some(cmd::tick(Duration::from_millis(100), Msg::Tick))
            }
            Msg::ShowConfirm => {
                self.confirm = Some(Confirm::new("Are you sure?"));
                None
            }
            Msg::AddToast => {
                let kinds = [ToastKind::Info, ToastKind::Success, ToastKind::Warning, ToastKind::Error];
                let kind = kinds[self.tick_count as usize % 4];
                self.toasts.push(kind, format!("Notification #{}", self.tick_count));
                None
            }
            Msg::Noop => None,
        }
    }

    fn view(&self) -> Element<Msg> {
        let header = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .bg(Color::gray(30))
                .child(Element::Text(
                    TextElement::new(" Widgets Demo").bold().fg(Color::White),
                ))
                .child(Element::Spacer)
                .child(Element::Text(
                    TextElement::new("t:toast c:confirm q:quit ").fg(Color::gray(150)),
                )),
        );

        let breadcrumb = Breadcrumb::new(vec!["Home", "Components", "Widgets"])
            .active_color(Color::Cyan)
            .element();

        let tree = Tree::new(TreeNode::branch("a3s-tui", vec![
            TreeNode::branch("src", vec![
                TreeNode::leaf("lib.rs"),
                TreeNode::leaf("element.rs"),
                TreeNode::branch("components", vec![
                    TreeNode::leaf("toast.rs"),
                    TreeNode::leaf("tree.rs"),
                ]),
            ]),
            TreeNode::leaf("Cargo.toml"),
        ]))
        .element();

        let kv = KeyValue::new()
            .pair("Version", "0.1.0")
            .pair("Components", "21")
            .pair("Tests", "184")
            .pair("Lines", "8,242")
            .key_color(Color::BrightBlack)
            .value_color(Color::Cyan)
            .element();

        let paragraph = Paragraph::new(
            "A3S TUI is a TEA framework for building terminal UIs with Flexbox layout, \
             incremental rendering, and a rich component library."
        )
        .width(50)
        .indent(2)
        .color(Color::gray(200))
        .element();

        let scrollbar = Scrollbar::new(100, 20, (self.tick_count as usize / 5) % 81)
            .thumb_color(Color::Cyan)
            .element(10);

        let mut content = col![
            header,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  ").fg(Color::White)),
            breadcrumb,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  File Tree:").bold().fg(Color::White)),
            tree,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  Stats:").bold().fg(Color::White)),
            kv,
            Element::Text(TextElement::new("")),
            Element::Text(TextElement::new("  About:").bold().fg(Color::White)),
            paragraph,
            Element::Text(TextElement::new("")),
            row![
                Element::Text(TextElement::new("  Scroll: ")),
                scrollbar,
            ],
        ];

        if !self.toasts.is_empty() {
            content = col![
                content,
                Element::Text(TextElement::new("")),
                self.toasts.element(),
            ];
        }

        if let Some(ref confirm) = self.confirm {
            content = col![
                content,
                Element::Text(TextElement::new("")),
                confirm.element(),
            ];
        }

        content
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = App {
        toasts: ToastManager::new().with_duration(Duration::from_secs(3)),
        confirm: None,
        tick_count: 0,
    };

    ElementProgramBuilder::new(app)
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
