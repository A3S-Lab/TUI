use a3s_tui::style::{Align, Border, Color, Style};

fn main() {
    let title = Style::new()
        .bold()
        .fg(Color::BrightCyan)
        .align(Align::Center)
        .width(50)
        .render("a3s-tui Style System Demo");

    let box1 = Style::new()
        .fg(Color::White)
        .bg(Color::Blue)
        .bold()
        .padding(1, 2)
        .border(Border::Rounded)
        .border_fg(Color::Cyan)
        .width(30)
        .render("Hello, World!\nThis is a styled box.");

    let box2 = Style::new()
        .fg(Color::Yellow)
        .italic()
        .padding(0, 1)
        .border(Border::Double)
        .border_fg(Color::Yellow)
        .align(Align::Center)
        .width(30)
        .render("Centered text\nin a double border");

    let box3 = Style::new()
        .dim()
        .strikethrough()
        .render("This text is dim and struck through");

    let box4 = Style::new()
        .fg(Color::Rgb(255, 165, 0))
        .underline()
        .margin(1, 0)
        .padding(0, 1)
        .border(Border::Thick)
        .border_fg(Color::Rgb(255, 100, 0))
        .render("TrueColor orange\nwith thick border");

    println!("{}\n", title);
    println!("{}\n", box1);
    println!("{}\n", box2);
    println!("{}\n", box3);
    println!("{}", box4);
}
