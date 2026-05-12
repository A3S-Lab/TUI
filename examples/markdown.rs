use a3s_tui::markdown::Markdown;

fn main() {
    let md = Markdown::new().with_width(70);

    let input = r#"# Welcome to a3s-tui

This is a **markdown renderer** with _italic_ and ~~strikethrough~~ support.

## Code Blocks

Here's some Rust code with syntax highlighting:

```rust
fn main() {
    let greeting = "Hello, World!";
    println!("{}", greeting);

    for i in 0..5 {
        println!("Count: {}", i);
    }
}
```

## Lists

- First item
- Second item with `inline code`
- Third item

## Blockquote

> This is a blockquote. It should be styled differently from normal text.

---

Check out [Rust](https://www.rust-lang.org) for more info.
"#;

    let rendered = md.render(input);
    println!("{}", rendered);
}
