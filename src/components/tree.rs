use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::{pad_visible, truncate_visible, Color, Style};

/// A tree node for hierarchical display.
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn branch(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        Self {
            label: label.into(),
            children,
        }
    }
}

/// A tree view component for displaying hierarchical data.
///
/// ```rust
/// use a3s_tui::components::tree::{Tree, TreeNode};
///
/// let tree = Tree::new(TreeNode::branch("root", vec![
///     TreeNode::leaf("file1.rs"),
///     TreeNode::branch("src", vec![
///         TreeNode::leaf("main.rs"),
///         TreeNode::leaf("lib.rs"),
///     ]),
/// ]));
/// ```
pub struct Tree {
    root: TreeNode,
    branch_color: Color,
    leaf_color: Color,
}

impl Tree {
    pub fn new(root: TreeNode) -> Self {
        Self {
            root,
            branch_color: Color::Blue,
            leaf_color: Color::White,
        }
    }

    pub fn branch_color(mut self, color: Color) -> Self {
        self.branch_color = color;
        self
    }

    pub fn leaf_color(mut self, color: Color) -> Self {
        self.leaf_color = color;
        self
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        let mut lines: Vec<Element<Msg>> = Vec::new();
        self.render_node(&self.root, &mut lines, "", true);
        Element::Box(
            BoxElement::new()
                .direction(FlexDirection::Column)
                .children(lines),
        )
    }

    pub fn view(&self, width: u16, height: usize) -> String {
        let width = width as usize;
        if width == 0 || height == 0 {
            return String::new();
        }

        let mut lines = Vec::new();
        self.render_node_lines(&self.root, &mut lines, "", true);
        lines
            .into_iter()
            .take(height)
            .map(|(line, color, bold)| {
                let clipped = truncate_visible(&line, width);
                let padded = pad_visible(&clipped, width);
                let style = if bold {
                    Style::new().fg(color).bold()
                } else {
                    Style::new().fg(color)
                };
                style.render(&padded)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_node<Msg>(
        &self,
        node: &TreeNode,
        lines: &mut Vec<Element<Msg>>,
        prefix: &str,
        is_root: bool,
    ) {
        let color = if node.children.is_empty() {
            self.leaf_color
        } else {
            self.branch_color
        };

        if is_root {
            lines.push(Element::Text(
                TextElement::new(&node.label).bold().fg(color),
            ));
        } else {
            let text = format!("{}{}", prefix, node.label);
            lines.push(Element::Text(TextElement::new(text).fg(color)));
        }

        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last = i == child_count - 1;
            let connector = if is_last { "└── " } else { "├── " };

            let display_prefix = if is_root {
                connector.to_string()
            } else {
                let base = prefix.replace("├── ", "│   ").replace("└── ", "    ");
                format!("{}{}", base, connector)
            };

            self.render_node(child, lines, &display_prefix, false);
        }
    }

    fn render_node_lines(
        &self,
        node: &TreeNode,
        lines: &mut Vec<(String, Color, bool)>,
        prefix: &str,
        is_root: bool,
    ) {
        let color = if node.children.is_empty() {
            self.leaf_color
        } else {
            self.branch_color
        };

        if is_root {
            lines.push((node.label.clone(), color, true));
        } else {
            lines.push((format!("{}{}", prefix, node.label), color, false));
        }

        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last = i == child_count - 1;
            let connector = if is_last { "└── " } else { "├── " };

            let display_prefix = if is_root {
                connector.to_string()
            } else {
                let base = prefix.replace("├── ", "│   ").replace("└── ", "    ");
                format!("{}{}", base, connector)
            };

            self.render_node_lines(child, lines, &display_prefix, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{strip_ansi, visible_len};

    #[test]
    fn single_leaf() {
        let tree = Tree::new(TreeNode::leaf("root"));
        let el: Element<()> = tree.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 1),
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn branch_with_children() {
        let tree = Tree::new(TreeNode::branch(
            "root",
            vec![TreeNode::leaf("a"), TreeNode::leaf("b")],
        ));
        let el: Element<()> = tree.element();
        match el {
            Element::Box(b) => assert_eq!(b.children.len(), 3), // root + 2 children
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn nested_tree() {
        let tree = Tree::new(TreeNode::branch(
            "project",
            vec![
                TreeNode::branch("src", vec![TreeNode::leaf("main.rs")]),
                TreeNode::leaf("Cargo.toml"),
            ],
        ));
        let el: Element<()> = tree.element();
        match el {
            Element::Box(b) => {
                // project + src + main.rs + Cargo.toml = 4
                assert_eq!(b.children.len(), 4);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn view_renders_connectors_and_padding() {
        let tree = Tree::new(TreeNode::branch(
            "root",
            vec![TreeNode::leaf("one"), TreeNode::leaf("two")],
        ));

        let rendered = tree.view(16, 4);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("root"));
        assert!(plain.contains("├── one"));
        assert!(plain.contains("└── two"));
        assert!(plain.lines().all(|line| visible_len(line) == 16));
    }

    #[test]
    fn view_truncates_and_limits_height() {
        let tree = Tree::new(TreeNode::branch(
            "root-with-a-long-name",
            vec![TreeNode::leaf("child"), TreeNode::leaf("hidden")],
        ));

        let rendered = tree.view(10, 2);
        let plain = strip_ansi(&rendered);
        let lines = plain.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains('…'));
        assert!(plain.contains("child"));
        assert!(!plain.contains("hidden"));
    }
}
