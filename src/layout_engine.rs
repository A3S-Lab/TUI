//! Flexbox layout engine powered by Taffy.
//!
//! Converts the Element tree into absolute positions using CSS Flexbox rules.

use crate::element::{
    AlignItems as ElAlignItems, BoxStyle, Dimension, Element, FlexDirection as ElFlexDir,
    JustifyContent as ElJustify, Overflow as ElOverflow, TextWrap,
};
use crate::style::{split_lines_preserving_trailing_blank, strip_ansi, visible_len, wrap_words};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use taffy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
}

#[derive(Debug, Clone)]
struct TextMeasureContext {
    content: String,
    wrap: TextWrap,
}

impl LayoutResult {
    fn fallback<Msg>(root: &Element<Msg>, available_width: u16, available_height: u16) -> Self {
        let (width, height) = fallback_root_size(root, available_width, available_height);
        Self {
            nodes: vec![LayoutNode {
                x: 0,
                y: 0,
                width,
                height,
            }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    Taffy(taffy::TaffyError),
    EnginePanic,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::Taffy(err) => write!(f, "taffy layout failed: {err}"),
            LayoutError::EnginePanic => write!(f, "taffy layout panicked"),
        }
    }
}

impl std::error::Error for LayoutError {}

impl From<taffy::TaffyError> for LayoutError {
    fn from(value: taffy::TaffyError) -> Self {
        Self::Taffy(value)
    }
}

pub struct LayoutEngine {
    tree: TaffyTree<TextMeasureContext>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    pub fn compute<Msg>(
        &mut self,
        root: &Element<Msg>,
        available_width: u16,
        available_height: u16,
    ) -> LayoutResult {
        self.try_compute(root, available_width, available_height)
            .unwrap_or_else(|_| LayoutResult::fallback(root, available_width, available_height))
    }

    pub fn try_compute<Msg>(
        &mut self,
        root: &Element<Msg>,
        available_width: u16,
        available_height: u16,
    ) -> Result<LayoutResult, LayoutError> {
        match catch_unwind(AssertUnwindSafe(|| {
            self.compute_taffy(root, available_width, available_height)
        })) {
            Ok(result) => result,
            Err(_) => {
                self.tree.clear();
                Err(LayoutError::EnginePanic)
            }
        }
    }

    fn compute_taffy<Msg>(
        &mut self,
        root: &Element<Msg>,
        available_width: u16,
        available_height: u16,
    ) -> Result<LayoutResult, LayoutError> {
        self.tree.clear();

        let mut nodes = Vec::new();
        let root_id = self.build_node(root, &mut nodes)?;

        let available = Size {
            width: AvailableSpace::Definite(available_width as f32),
            height: AvailableSpace::Definite(available_height as f32),
        };

        self.tree.compute_layout_with_measure(
            root_id,
            available,
            |known_dimensions, available_space, _node_id, context, _style| {
                let Some(context) = context else {
                    return Size::ZERO;
                };
                measure_text_node(
                    &context.content,
                    context.wrap,
                    known_dimensions,
                    available_space,
                )
            },
        )?;

        let mut result_nodes = Vec::with_capacity(nodes.len());
        self.collect_layout(root_id, 0.0, 0.0, &mut result_nodes)?;

        Ok(LayoutResult {
            nodes: result_nodes,
        })
    }

    fn build_node<Msg>(
        &mut self,
        element: &Element<Msg>,
        node_list: &mut Vec<NodeId>,
    ) -> Result<NodeId, LayoutError> {
        match element {
            Element::Box(box_el) => {
                let child_ids: Vec<NodeId> = box_el
                    .children
                    .iter()
                    .map(|child| self.build_node(child, node_list))
                    .collect::<Result<_, _>>()?;

                let style = self.box_style_to_taffy(&box_el.style);
                let node_id = self.tree.new_with_children(style, &child_ids)?;
                node_list.push(node_id);
                Ok(node_id)
            }
            Element::Text(text_el) => {
                let style = Style {
                    flex_shrink: 1.0,
                    ..Default::default()
                };

                let node_id = self.tree.new_leaf_with_context(
                    style,
                    TextMeasureContext {
                        content: text_el.content.clone(),
                        wrap: text_el.wrap,
                    },
                )?;
                node_list.push(node_id);
                Ok(node_id)
            }
            Element::Spacer => {
                let style = Style {
                    flex_grow: 1.0,
                    ..Default::default()
                };
                let node_id = self.tree.new_leaf(style)?;
                node_list.push(node_id);
                Ok(node_id)
            }
            Element::_Phantom(_) => {
                let node_id = self.tree.new_leaf(Style::default())?;
                node_list.push(node_id);
                Ok(node_id)
            }
        }
    }

    fn box_style_to_taffy(&self, bs: &BoxStyle) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: match bs.flex_direction {
                ElFlexDir::Row => taffy::FlexDirection::Row,
                ElFlexDir::Column => taffy::FlexDirection::Column,
            },
            align_items: Some(match bs.align_items {
                ElAlignItems::Start => taffy::AlignItems::FlexStart,
                ElAlignItems::Center => taffy::AlignItems::Center,
                ElAlignItems::End => taffy::AlignItems::FlexEnd,
                ElAlignItems::Stretch => taffy::AlignItems::Stretch,
            }),
            justify_content: Some(match bs.justify_content {
                ElJustify::Start => taffy::JustifyContent::FlexStart,
                ElJustify::Center => taffy::JustifyContent::Center,
                ElJustify::End => taffy::JustifyContent::FlexEnd,
                ElJustify::SpaceBetween => taffy::JustifyContent::SpaceBetween,
                ElJustify::SpaceAround => taffy::JustifyContent::SpaceAround,
                ElJustify::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
            }),
            gap: Size {
                width: LengthPercentage::Length(bs.gap as f32),
                height: LengthPercentage::Length(bs.gap as f32),
            },
            padding: Rect {
                top: LengthPercentage::Length(bs.padding.top as f32),
                right: LengthPercentage::Length(bs.padding.right as f32),
                bottom: LengthPercentage::Length(bs.padding.bottom as f32),
                left: LengthPercentage::Length(bs.padding.left as f32),
            },
            margin: Rect {
                top: LengthPercentageAuto::Length(bs.margin.top as f32),
                right: LengthPercentageAuto::Length(bs.margin.right as f32),
                bottom: LengthPercentageAuto::Length(bs.margin.bottom as f32),
                left: LengthPercentageAuto::Length(bs.margin.left as f32),
            },
            size: Size {
                width: dim_to_taffy(bs.width),
                height: dim_to_taffy(bs.height),
            },
            min_size: Size {
                width: dim_to_taffy(bs.min_width),
                height: dim_to_taffy(bs.min_height),
            },
            max_size: Size {
                width: dim_to_taffy(bs.max_width),
                height: dim_to_taffy(bs.max_height),
            },
            flex_grow: non_negative_finite(bs.flex_grow),
            flex_shrink: non_negative_finite(bs.flex_shrink),
            flex_basis: dim_to_taffy(bs.flex_basis),
            overflow: taffy::Point {
                x: match bs.overflow {
                    ElOverflow::Visible => taffy::Overflow::Visible,
                    ElOverflow::Hidden => taffy::Overflow::Hidden,
                },
                y: match bs.overflow {
                    ElOverflow::Visible => taffy::Overflow::Visible,
                    ElOverflow::Hidden => taffy::Overflow::Hidden,
                },
            },
            ..Default::default()
        }
    }

    fn collect_layout(
        &self,
        node_id: NodeId,
        parent_x: f32,
        parent_y: f32,
        result: &mut Vec<LayoutNode>,
    ) -> Result<(), LayoutError> {
        let layout = self.tree.layout(node_id)?;
        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;

        result.push(LayoutNode {
            x: layout_value_to_u16(x),
            y: layout_value_to_u16(y),
            width: layout_value_to_u16(layout.size.width),
            height: layout_value_to_u16(layout.size.height),
        });

        let children = self.tree.children(node_id)?;
        for child_id in children {
            self.collect_layout(child_id, x, y, result)?;
        }
        Ok(())
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn dim_to_taffy(d: Dimension) -> taffy::Dimension {
    match d {
        Dimension::Auto => taffy::Dimension::Auto,
        Dimension::Points(p) => taffy::Dimension::Length(non_negative_finite(p)),
        Dimension::Percent(p) => {
            taffy::Dimension::Percent(non_negative_finite(p).min(100.0) / 100.0)
        }
    }
}

fn non_negative_finite(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn layout_value_to_u16(value: f32) -> u16 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= u16::MAX as f32 {
        u16::MAX
    } else {
        value as u16
    }
}

fn fallback_root_size<Msg>(
    root: &Element<Msg>,
    available_width: u16,
    available_height: u16,
) -> (u16, u16) {
    match root {
        Element::Text(text_el) => {
            let measured = measure_text_node(
                &text_el.content,
                text_el.wrap,
                Size {
                    width: None,
                    height: None,
                },
                Size {
                    width: AvailableSpace::Definite(available_width as f32),
                    height: AvailableSpace::Definite(available_height as f32),
                },
            );
            let width = layout_value_to_u16(measured.width).min(available_width);
            let height = layout_value_to_u16(measured.height).min(available_height);
            (width, height)
        }
        Element::Box(_) | Element::Spacer | Element::_Phantom(_) => {
            (available_width, available_height)
        }
    }
}

fn measure_text_node(
    content: &str,
    wrap: TextWrap,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    let lines = split_lines_preserving_trailing_blank(content);

    let max_line_width = lines.iter().map(|l| visible_len(l)).max().unwrap_or(0) as f32;

    match wrap {
        TextWrap::NoWrap | TextWrap::Truncate => Size {
            width: known.width.unwrap_or(max_line_width),
            height: known.height.unwrap_or(lines.len() as f32),
        },
        TextWrap::Wrap => {
            let avail_w = match available.width {
                AvailableSpace::Definite(w) => w,
                _ => max_line_width,
            };
            let w = known.width.unwrap_or(max_line_width.min(avail_w));
            let wrapped_lines = count_wrapped_lines(content, w as usize);
            Size {
                width: w,
                height: known.height.unwrap_or(wrapped_lines as f32),
            }
        }
    }
}

fn count_wrapped_lines(text: &str, width: usize) -> usize {
    if width == 0 {
        return split_lines_preserving_trailing_blank(text).len().max(1);
    }

    let stripped_text;
    let text = if text.contains('\x1b') {
        stripped_text = strip_ansi(text);
        stripped_text.as_str()
    } else {
        text
    };

    split_lines_preserving_trailing_blank(text)
        .into_iter()
        .map(|line| wrap_words(line, width).len())
        .sum::<usize>()
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{BoxElement, Element, FlexDirection, TextElement};

    #[test]
    fn layout_single_text() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Text(TextElement::new("Hello"));
        let result = engine.compute(&el, 80, 24);
        assert!(!result.nodes.is_empty());
        assert_eq!(result.nodes[0].width, 5);
        assert_eq!(result.nodes[0].height, 1);
    }

    #[test]
    fn layout_wrap_text_uses_available_width_for_height() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Text(TextElement::new("alpha beta"));

        let result = engine.compute(&el, 6, 2);

        assert_eq!(result.nodes[0].width, 6);
        assert_eq!(result.nodes[0].height, 2);
    }

    #[test]
    fn layout_wrap_text_height_matches_word_wrapping() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Text(TextElement::new("a b c d e f"));

        let result = engine.compute(&el, 3, 5);

        assert_eq!(result.nodes[0].width, 3);
        assert_eq!(result.nodes[0].height, 3);
    }

    #[test]
    fn layout_text_preserves_trailing_blank_line_height() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Text(TextElement::new("Line\n"));

        let result = engine.compute(&el, 80, 24);

        assert_eq!(result.nodes[0].height, 2);
    }

    #[test]
    fn count_wrapped_lines_ignores_ansi_sequences() {
        let styled = "\x1b[31malpha beta\x1b[0m";

        assert_eq!(count_wrapped_lines(styled, 6), 2);
    }

    #[test]
    fn layout_column_stacks_vertically() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .child(Element::Text(TextElement::new("Line 1")))
                .child(Element::Text(TextElement::new("Line 2"))),
        );
        let result = engine.compute(&el, 80, 24);
        assert!(result.nodes.len() >= 3);
        let child1 = &result.nodes[1];
        let child2 = &result.nodes[2];
        assert!(child2.y > child1.y);
    }

    #[test]
    fn layout_row_places_side_by_side() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new("Left")))
                .child(Element::Text(TextElement::new("Right"))),
        );
        let result = engine.compute(&el, 80, 24);
        assert!(result.nodes.len() >= 3);
        let child1 = &result.nodes[1];
        let child2 = &result.nodes[2];
        assert!(child2.x > child1.x);
        assert_eq!(child1.y, child2.y);
    }

    #[test]
    fn layout_spacer_expands() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .width(Dimension::Points(80.0))
                .child(Element::Text(TextElement::new("L")))
                .child(Element::Spacer)
                .child(Element::Text(TextElement::new("R"))),
        );
        let result = engine.compute(&el, 80, 24);
        assert!(result.nodes.len() >= 4);
        let left = &result.nodes[1];
        let right = &result.nodes[3];
        assert_eq!(left.x, 0);
        assert_eq!(right.x, 79);
    }

    #[test]
    fn layout_padding_offsets_children() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .padding(2)
                .child(Element::Text(TextElement::new("Hi"))),
        );
        let result = engine.compute(&el, 80, 24);
        let child = &result.nodes[1];
        assert_eq!(child.x, 2);
        assert_eq!(child.y, 2);
    }

    #[test]
    fn layout_gap_adds_spacing() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .gap(1)
                .child(Element::Text(TextElement::new("A")))
                .child(Element::Text(TextElement::new("B"))),
        );
        let result = engine.compute(&el, 80, 24);
        let child1 = &result.nodes[1];
        let child2 = &result.nodes[2];
        assert_eq!(child2.y - child1.y, 2);
    }

    #[test]
    fn try_compute_returns_layout_for_valid_tree() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Row)
                .child(Element::Text(TextElement::new("Left")))
                .child(Element::Text(TextElement::new("Right"))),
        );

        let result = engine
            .try_compute(&el, 80, 24)
            .expect("valid layout should compute");

        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.nodes[0].x, 0);
        assert_eq!(result.nodes[0].y, 0);
    }

    #[test]
    fn compute_sanitizes_non_finite_style_values() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .width(Dimension::Points(f32::NAN))
                .height(Dimension::Points(f32::INFINITY))
                .flex_grow(f32::NAN)
                .flex_shrink(f32::NEG_INFINITY)
                .child(Element::Text(TextElement::new("content"))),
        );

        let result = engine.compute(&el, 80, 24);

        assert_eq!(result.nodes[0].width, 0);
        assert_eq!(result.nodes[0].height, 0);
    }

    #[test]
    fn compute_clamps_oversized_percent_dimensions() {
        let mut engine = LayoutEngine::new();
        let el: Element<()> = Element::Box(
            BoxElement::new()
                .width(Dimension::Percent(250.0))
                .height(Dimension::Percent(125.0)),
        );

        let result = engine.compute(&el, 80, 24);

        assert_eq!(result.nodes[0].width, 80);
        assert_eq!(result.nodes[0].height, 24);
    }

    #[test]
    fn compute_applies_box_min_and_max_height_builders() {
        let mut engine = LayoutEngine::new();
        let min_el: Element<()> = Element::Box(
            BoxElement::new()
                .height(Dimension::Points(1.0))
                .min_height(Dimension::Points(3.0)),
        );
        let max_el: Element<()> = Element::Box(
            BoxElement::new()
                .height(Dimension::Points(5.0))
                .max_height(Dimension::Points(2.0)),
        );

        let min_result = engine.compute(&min_el, 80, 24);
        let max_result = engine.compute(&max_el, 80, 24);

        assert_eq!(min_result.nodes[0].height, 3);
        assert_eq!(max_result.nodes[0].height, 2);
    }

    #[test]
    fn fallback_layout_bounds_text_root_to_available_area() {
        let el: Element<()> = Element::Text(TextElement::new("abcdef\nxy"));
        let result = LayoutResult::fallback(&el, 3, 1);

        assert_eq!(
            result.nodes,
            vec![LayoutNode {
                x: 0,
                y: 0,
                width: 3,
                height: 1,
            }]
        );
    }

    #[test]
    fn fallback_layout_wraps_text_root_to_available_width() {
        let el: Element<()> = Element::Text(TextElement::new("a b c d e f"));
        let result = LayoutResult::fallback(&el, 3, 5);

        assert_eq!(
            result.nodes,
            vec![LayoutNode {
                x: 0,
                y: 0,
                width: 3,
                height: 3,
            }]
        );
    }

    #[test]
    fn layout_value_to_u16_clamps_non_finite_and_large_values() {
        assert_eq!(layout_value_to_u16(f32::NAN), 0);
        assert_eq!(layout_value_to_u16(f32::INFINITY), 0);
        assert_eq!(layout_value_to_u16(-1.0), 0);
        assert_eq!(layout_value_to_u16(8.9), 8);
        assert_eq!(layout_value_to_u16(100_000.0), u16::MAX);
    }
}
