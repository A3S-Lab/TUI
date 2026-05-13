/// Column layout (flex-direction: column)
#[macro_export]
macro_rules! col {
    [$($child:expr),* $(,)?] => {
        $crate::Element::Box($crate::BoxElement::new()
            .direction($crate::FlexDirection::Column)
            .children(vec![$($child),*]))
    };
}

/// Row layout (flex-direction: row)
#[macro_export]
macro_rules! row {
    [$($child:expr),* $(,)?] => {
        $crate::Element::Box($crate::BoxElement::new()
            .direction($crate::FlexDirection::Row)
            .children(vec![$($child),*]))
    };
}

/// Text element shorthand
#[macro_export]
macro_rules! text {
    ($content:expr) => {
        $crate::Element::Text($crate::TextElement::new($content))
    };
}

/// Spacer shorthand
#[macro_export]
macro_rules! spacer {
    () => {
        $crate::Element::Spacer
    };
}

#[cfg(test)]
mod tests {
    use crate::element::{Element, FlexDirection};

    #[test]
    fn col_macro_creates_column() {
        let el: Element<()> = col![
            text!("a"),
            text!("b"),
        ];
        match el {
            Element::Box(b) => {
                assert_eq!(b.style.flex_direction, FlexDirection::Column);
                assert_eq!(b.children.len(), 2);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn row_macro_creates_row() {
        let el: Element<()> = row![
            text!("x"),
            text!("y"),
            text!("z"),
        ];
        match el {
            Element::Box(b) => {
                assert_eq!(b.style.flex_direction, FlexDirection::Row);
                assert_eq!(b.children.len(), 3);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn text_macro_creates_text() {
        let el: Element<()> = text!("hello");
        match el {
            Element::Text(t) => assert_eq!(t.content, "hello"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn spacer_macro_creates_spacer() {
        let el: Element<()> = spacer!();
        assert!(matches!(el, Element::Spacer));
    }

    #[test]
    fn nested_macros() {
        let el: Element<()> = col![
            row![
                text!("left"),
                spacer!(),
                text!("right"),
            ],
            text!("bottom"),
        ];
        match el {
            Element::Box(b) => {
                assert_eq!(b.children.len(), 2);
                match &b.children[0] {
                    Element::Box(inner) => {
                        assert_eq!(inner.style.flex_direction, FlexDirection::Row);
                        assert_eq!(inner.children.len(), 3);
                    }
                    _ => panic!("expected nested Box"),
                }
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn empty_col() {
        let el: Element<()> = col![];
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 0),
            _ => panic!("expected Box"),
        }
    }
}
