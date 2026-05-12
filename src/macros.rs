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
