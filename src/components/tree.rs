use crate::element::{BoxElement, Element, FlexDirection, TextElement};
use crate::style::Color;

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
            lines.push(Element::Text(TextElement::new(&node.label).bold().fg(color)));
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
