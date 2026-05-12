use a3s_tui::layout::{Constraint, Layout};
use a3s_tui::style::{Border, Color, Style};

fn main() {
    let width: u16 = 60;
    let height: u16 = 20;

    let header = Style::new()
        .bold()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .width(width)
        .render(" a3s-tui Layout Demo");

    let sidebar = Style::new()
        .fg(Color::Yellow)
        .border(Border::Single)
        .border_fg(Color::BrightBlack)
        .render("Sidebar\n\n- Item 1\n- Item 2\n- Item 3\n- Item 4");

    let main_content = Style::new()
        .fg(Color::White)
        .border(Border::Rounded)
        .border_fg(Color::Blue)
        .render("Main Content\n\nThis is the primary\ncontent area that\ntakes up most of\nthe available space.");

    let footer = Style::new()
        .dim()
        .fg(Color::White)
        .bg(Color::BrightBlack)
        .width(width)
        .render(" Status: Ready | q to quit");

    // Horizontal split for sidebar + main
    let body = Layout::horizontal()
        .item(&sidebar, Constraint::Fixed(20))
        .item(&main_content, Constraint::Fill)
        .render(width);

    // Vertical layout: header | body | footer
    let full = Layout::vertical()
        .item(&header, Constraint::Fixed(1))
        .item(&body, Constraint::Fill)
        .item(&footer, Constraint::Fixed(1))
        .render(height);

    println!("{}", full);
}
