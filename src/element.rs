use crate::style::Color;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Dimension {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextWrap {
    Wrap,
    Truncate,
    NoWrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Single,
    Double,
    Rounded,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Edges {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Edges {
    pub fn all(v: u16) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }
    pub fn xy(x: u16, y: u16) -> Self {
        Self { top: y, right: x, bottom: y, left: x }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub strikethrough: bool,
}

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


pub enum Element<Msg> {
    Box(BoxElement<Msg>),
    Text(TextElement),
    Spacer,
    _Phantom(PhantomData<Msg>),
}

pub struct BoxElement<Msg> {
    pub children: Vec<Element<Msg>>,
    pub style: BoxStyle,
    _phantom: PhantomData<Msg>,
}

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