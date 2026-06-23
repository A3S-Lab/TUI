//! Flexbox layout engine powered by Taffy.
//!
//! Converts the Element tree into absolute positions using CSS Flexbox rules.

use crate::element::{
    AlignItems as ElAlignItems, BoxStyle, Dimension, Element, FlexDirection as ElFlexDir,
    JustifyContent as ElJustify, Overflow as ElOverflow, TextWrap,
};
use crate::style::visible_len;
use taffy::prelude::*;

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
}

pub struct LayoutEngine {
    tree: TaffyTree<()>,
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
        self.tree.clear();

        let mut nodes = Vec::new();
        let root_id = self.build_node(root, &mut nodes);

        let available = Size {
            width: AvailableSpace::Definite(available_width as f32),
            height: AvailableSpace::Definite(available_height as f32),
        };

        self.tree.compute_layout(root_id, available).ok();

        let mut result_nodes = Vec::with_capacity(nodes.len());
        self.collect_layout(&nodes, root_id, 0.0, 0.0, &mut result_nodes);

        LayoutResult {
            nodes: result_nodes,
        }
    }

    fn build_node<Msg>(&mut self, element: &Element<Msg>, node_list: &mut Vec<NodeId>) -> NodeId {
        match element {
            Element::Box(box_el) => {
                let child_ids: Vec<NodeId> = box_el
                    .children
                    .iter()
                    .map(|child| self.build_node(child, node_list))
                    .collect();

                let style = self.box_style_to_taffy(&box_el.style);
                let node_id = self.tree.new_with_children(style, &child_ids).unwrap();
                node_list.push(node_id);
                node_id
            }
            Element::Text(text_el) => {
                let lines: Vec<&str> = text_el.content.lines().collect();
                let lines = if lines.is_empty() { vec![""] } else { lines };
                let max_w = lines.iter().map(|l| visible_len(l)).max().unwrap_or(0) as f32;
                let h = lines.len() as f32;

                let style = Style {
                    flex_shrink: 1.0,
                    size: Size {
                        width: taffy::Dimension::Length(max_w),
                        height: taffy::Dimension::Length(h),
                    },
                    ..Default::default()
                };

                let node_id = self.tree.new_leaf(style).unwrap();
                node_list.push(node_id);
                node_id
            }
            Element::Spacer => {
                let style = Style {
                    flex_grow: 1.0,
                    ..Default::default()
                };
                let node_id = self.tree.new_leaf(style).unwrap();
                node_list.push(node_id);
                node_id
            }
            Element::_Phantom(_) => {
                let node_id = self.tree.new_leaf(Style::default()).unwrap();
                node_list.push(node_id);
                node_id
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
            flex_grow: bs.flex_grow,
            flex_shrink: bs.flex_shrink,
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
        _node_list: &[NodeId],
        node_id: NodeId,
        parent_x: f32,
        parent_y: f32,
        result: &mut Vec<LayoutNode>,
    ) {
        let layout = self.tree.layout(node_id).unwrap();
        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;

        result.push(LayoutNode {
            x: x as u16,
            y: y as u16,
            width: layout.size.width as u16,
            height: layout.size.height as u16,
        });

        let children = self.tree.children(node_id).unwrap_or_default();
        for child_id in children {
            self.collect_layout(_node_list, child_id, x, y, result);
        }
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
        Dimension::Points(p) => taffy::Dimension::Length(p),
        Dimension::Percent(p) => taffy::Dimension::Percent(p / 100.0),
    }
}

#[allow(dead_code)]
fn measure_text_node(
    content: &str,
    wrap: TextWrap,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    let lines: Vec<&str> = content.lines().collect();
    let lines = if lines.is_empty() { vec![""] } else { lines };

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

#[allow(dead_code)]
fn count_wrapped_lines(text: &str, width: usize) -> usize {
    if width == 0 {
        return text.lines().count().max(1);
    }
    let mut total = 0;
    for line in text.lines() {
        let line_width = visible_len(line);
        if line_width == 0 {
            total += 1;
        } else {
            total += line_width.div_ceil(width);
        }
    }
    total.max(1)
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
}
