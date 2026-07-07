//! Element tree and styling primitives for declarative UI.
//!
//! This module provides the core building blocks for constructing terminal UIs:
//! - [`Element`]: The main UI tree structure (Box, Text, Spacer)
//! - [`BoxElement`]: Container with Flexbox layout
//! - [`TextElement`]: Styled text content
//! - Layout enums: [`FlexDirection`], [`AlignItems`], [`JustifyContent`]
//! - Styling types: [`Dimension`], [`Edges`], [`BorderStyle`]

use crate::style::Color;
use std::marker::PhantomData;

/// Flexbox layout direction (row or column).
///
/// # Examples
///
/// ```
/// use a3s_tui::element::{BoxElement, FlexDirection};
///
/// let row: BoxElement<()> = BoxElement::new().direction(FlexDirection::Row);
/// let col: BoxElement<()> = BoxElement::new().direction(FlexDirection::Column);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    /// Horizontal layout (left to right).
    #[default]
    Row,
    /// Vertical layout (top to bottom).
    Column,
}

/// Cross-axis alignment for flex items.
///
/// Controls how items are aligned perpendicular to the flex direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    /// Align to the start of the cross axis.
    Start,
    /// Center items on the cross axis.
    Center,
    /// Align to the end of the cross axis.
    End,
    /// Stretch items to fill the cross axis (default).
    #[default]
    Stretch,
}

/// Main-axis alignment for flex items.
///
/// Controls how items are distributed along the flex direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    /// Pack items at the start (default).
    #[default]
    Start,
    /// Center items.
    Center,
    /// Pack items at the end.
    End,
    /// Distribute items with space between them.
    SpaceBetween,
    /// Distribute items with space around them.
    SpaceAround,
    /// Distribute items with equal space around them.
    SpaceEvenly,
}

/// Size dimension (auto, fixed points, or percentage).
///
/// # Examples
///
/// ```
/// use a3s_tui::element::Dimension;
///
/// let auto = Dimension::Auto;
/// let fixed = Dimension::Points(100.0);
/// let percent = Dimension::Percent(50.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Dimension {
    /// Automatically calculate size based on content.
    #[default]
    Auto,
    /// Fixed size in terminal cells.
    Points(f32),
    /// Percentage of parent size (0.0 to 100.0).
    Percent(f32),
}

/// Overflow behavior for content that exceeds container bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// Content overflows visibly (default).
    #[default]
    Visible,
    /// Clip content at container bounds.
    Hidden,
}

/// Text wrapping behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextWrap {
    /// Wrap text at word boundaries.
    Wrap,
    /// Truncate text with ellipsis.
    Truncate,
    /// No wrapping (overflow).
    NoWrap,
}

/// Border style for box elements.
///
/// # Examples
///
/// ```
/// use a3s_tui::element::{BoxElement, BorderStyle};
///
/// let box_elem: BoxElement<()> = BoxElement::new().border(BorderStyle::Rounded);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// Single-line border (─│┌┐└┘).
    Single,
    /// Double-line border (═║╔╗╚╝).
    Double,
    /// Rounded corners (─│╭╮╰╯).
    Rounded,
    /// Thick border (━┃┏┓┗┛).
    Thick,
}

/// Spacing values for padding and margin (top, right, bottom, left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Edges {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Edges {
    /// Set all four edges to the same value.
    pub fn all(v: u16) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
    /// Set horizontal (x) and vertical (y) edges separately.
    pub fn xy(x: u16, y: u16) -> Self {
        Self {
            top: y,
            right: x,
            bottom: y,
            left: x,
        }
    }
}

/// Text styling properties (color, weight, decoration).
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
    pub dim: bool,
    pub strikethrough: bool,
}

/// Flexbox container style properties.
#[derive(Debug, Clone, Default)]
pub struct BoxStyle {
    pub flex_direction: FlexDirection,
    pub padding: Edges,
    pub margin: Edges,
    pub border: Option<BorderStyle>,
    pub border_color: Option<Color>,
    pub gap: u16,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub width: Dimension,
    pub height: Dimension,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,
    pub bg: Option<Color>,
    pub overflow: Overflow,
}

/// A node in the UI element tree.
///
/// Elements are the building blocks of the declarative UI. Use `col![]` and `row![]`
/// macros for concise construction.
pub enum Element<Msg> {
    /// A container with Flexbox layout.
    Box(BoxElement<Msg>),
    /// Styled text content.
    Text(TextElement),
    /// Flexible space that expands to fill available room.
    Spacer,
    _Phantom(PhantomData<Msg>),
}

impl<Msg> Element<Msg> {
    /// Create a text element.
    pub fn text(content: impl Into<String>) -> Self {
        Element::Text(TextElement::new(content))
    }

    /// Check if this is a Box element.
    pub fn is_box(&self) -> bool {
        matches!(self, Element::Box(_))
    }

    /// Check if this is a Text element.
    pub fn is_text(&self) -> bool {
        matches!(self, Element::Text(_))
    }

    /// Check if this is a Spacer.
    pub fn is_spacer(&self) -> bool {
        matches!(self, Element::Spacer)
    }

    /// Get the text content if this is a Text element.
    pub fn text_content(&self) -> Option<&str> {
        match self {
            Element::Text(t) => Some(&t.content),
            _ => None,
        }
    }

    /// Get the children if this is a Box element.
    pub fn children(&self) -> Option<&[Element<Msg>]> {
        match self {
            Element::Box(b) => Some(&b.children),
            _ => None,
        }
    }

    /// Count total children (0 for non-Box elements).
    pub fn child_count(&self) -> usize {
        match self {
            Element::Box(b) => b.children.len(),
            _ => 0,
        }
    }
}

/// A Flexbox container element that holds child elements.
///
/// Use the builder pattern to configure layout and styling:
///
/// ```
/// use a3s_tui::element::{BoxElement, FlexDirection, BorderStyle, Dimension};
///
/// let container = BoxElement::<()>::new()
///     .direction(FlexDirection::Column)
///     .padding(1)
///     .gap(1)
///     .border(BorderStyle::Rounded)
///     .width(Dimension::Percent(100.0));
/// ```
pub struct BoxElement<Msg> {
    pub children: Vec<Element<Msg>>,
    pub style: BoxStyle,
    _phantom: PhantomData<Msg>,
}

/// A styled text element.
///
/// ```
/// use a3s_tui::element::TextElement;
/// use a3s_tui::style::Color;
///
/// let text = TextElement::new("Hello, world!")
///     .bold()
///     .fg(Color::Cyan);
/// ```
pub struct TextElement {
    pub content: String,
    pub style: TextStyle,
    pub wrap: TextWrap,
}

impl<Msg> BoxElement<Msg> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: BoxStyle::default(),
            _phantom: PhantomData,
        }
    }

    /// Create a row container (flex-direction: row).
    pub fn row() -> Self {
        Self::new().direction(FlexDirection::Row)
    }

    /// Create a column container (flex-direction: column).
    pub fn col() -> Self {
        Self::new().direction(FlexDirection::Column)
    }

    pub fn direction(mut self, d: FlexDirection) -> Self {
        self.style.flex_direction = d;
        self
    }

    pub fn child(mut self, el: Element<Msg>) -> Self {
        self.children.push(el);
        self
    }

    pub fn children(mut self, els: Vec<Element<Msg>>) -> Self {
        self.children = els;
        self
    }

    pub fn padding(mut self, all: u16) -> Self {
        self.style.padding = Edges::all(all);
        self
    }

    pub fn padding_xy(mut self, x: u16, y: u16) -> Self {
        self.style.padding = Edges::xy(x, y);
        self
    }

    pub fn margin(mut self, all: u16) -> Self {
        self.style.margin = Edges::all(all);
        self
    }

    pub fn border(mut self, style: BorderStyle) -> Self {
        self.style.border = Some(style);
        self
    }

    pub fn border_color(mut self, c: Color) -> Self {
        self.style.border_color = Some(c);
        self
    }

    pub fn gap(mut self, g: u16) -> Self {
        self.style.gap = g;
        self
    }

    pub fn flex_grow(mut self, g: f32) -> Self {
        self.style.flex_grow = g;
        self
    }

    pub fn flex_shrink(mut self, s: f32) -> Self {
        self.style.flex_shrink = s;
        self
    }

    pub fn flex_basis(mut self, d: Dimension) -> Self {
        self.style.flex_basis = d;
        self
    }

    pub fn width(mut self, d: Dimension) -> Self {
        self.style.width = d;
        self
    }

    pub fn height(mut self, d: Dimension) -> Self {
        self.style.height = d;
        self
    }

    pub fn min_width(mut self, d: Dimension) -> Self {
        self.style.min_width = d;
        self
    }

    pub fn max_width(mut self, d: Dimension) -> Self {
        self.style.max_width = d;
        self
    }

    pub fn min_height(mut self, d: Dimension) -> Self {
        self.style.min_height = d;
        self
    }

    pub fn max_height(mut self, d: Dimension) -> Self {
        self.style.max_height = d;
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.style.bg = Some(c);
        self
    }

    pub fn align_items(mut self, a: AlignItems) -> Self {
        self.style.align_items = a;
        self
    }

    pub fn justify_content(mut self, j: JustifyContent) -> Self {
        self.style.justify_content = j;
        self
    }

    pub fn overflow(mut self, o: Overflow) -> Self {
        self.style.overflow = o;
        self
    }
}

impl<Msg> Default for BoxElement<Msg> {
    fn default() -> Self {
        Self::new()
    }
}

impl TextElement {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: TextStyle::default(),
            wrap: TextWrap::Wrap,
        }
    }

    pub fn fg(mut self, c: Color) -> Self {
        self.style.fg = Some(c);
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.style.bg = Some(c);
        self
    }

    pub fn bold(mut self) -> Self {
        self.style.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.style.underline = true;
        self
    }

    pub fn reverse(mut self) -> Self {
        self.style.reverse = true;
        self
    }

    pub fn dim(mut self) -> Self {
        self.style.dim = true;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.style.strikethrough = true;
        self
    }

    pub fn wrap(mut self, w: TextWrap) -> Self {
        self.wrap = w;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_all() {
        let e = Edges::all(5);
        assert_eq!(e.top, 5);
        assert_eq!(e.right, 5);
        assert_eq!(e.bottom, 5);
        assert_eq!(e.left, 5);
    }

    #[test]
    fn edges_xy() {
        let e = Edges::xy(3, 7);
        assert_eq!(e.left, 3);
        assert_eq!(e.right, 3);
        assert_eq!(e.top, 7);
        assert_eq!(e.bottom, 7);
    }

    #[test]
    fn text_element_builder() {
        let t = TextElement::new("hello")
            .bold()
            .italic()
            .reverse()
            .fg(Color::Red)
            .bg(Color::Blue);
        assert_eq!(t.content, "hello");
        assert!(t.style.bold);
        assert!(t.style.italic);
        assert!(t.style.reverse);
        assert_eq!(t.style.fg, Some(Color::Red));
        assert_eq!(t.style.bg, Some(Color::Blue));
    }

    #[test]
    fn box_element_builder() {
        let b = BoxElement::<()>::new()
            .direction(FlexDirection::Column)
            .padding(2)
            .gap(1)
            .border(BorderStyle::Rounded)
            .bg(Color::Black);
        assert_eq!(b.style.flex_direction, FlexDirection::Column);
        assert_eq!(b.style.padding, Edges::all(2));
        assert_eq!(b.style.gap, 1);
        assert_eq!(b.style.border, Some(BorderStyle::Rounded));
        assert_eq!(b.style.bg, Some(Color::Black));
    }

    #[test]
    fn box_element_children() {
        let b = BoxElement::<()>::new()
            .child(Element::Text(TextElement::new("a")))
            .child(Element::Text(TextElement::new("b")))
            .child(Element::Spacer);
        assert_eq!(b.children.len(), 3);
    }

    #[test]
    fn box_element_flex_properties() {
        let b = BoxElement::<()>::new()
            .flex_grow(2.0)
            .flex_shrink(0.5)
            .flex_basis(Dimension::Points(20.0))
            .width(Dimension::Points(100.0))
            .height(Dimension::Percent(50.0))
            .min_width(Dimension::Points(10.0))
            .max_width(Dimension::Points(120.0))
            .min_height(Dimension::Points(4.0))
            .max_height(Dimension::Points(60.0));
        assert_eq!(b.style.flex_grow, 2.0);
        assert_eq!(b.style.flex_shrink, 0.5);
        assert_eq!(b.style.flex_basis, Dimension::Points(20.0));
        assert_eq!(b.style.width, Dimension::Points(100.0));
        assert_eq!(b.style.height, Dimension::Percent(50.0));
        assert_eq!(b.style.min_width, Dimension::Points(10.0));
        assert_eq!(b.style.max_width, Dimension::Points(120.0));
        assert_eq!(b.style.min_height, Dimension::Points(4.0));
        assert_eq!(b.style.max_height, Dimension::Points(60.0));
    }
}
