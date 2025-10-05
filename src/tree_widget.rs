use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{StatefulWidget, Widget},
};

pub struct TreeNode {
    id: usize,
    text: String,
    children: Vec<TreeNode>,
    parent_id: Option<usize>,
}

impl TreeNode {
    pub fn new(id: usize, text: String) -> Self {
        Self {
            id,
            text,
            children: Vec::new(),
            parent_id: None,
        }
    }

    pub fn add_child(&mut self, mut child: TreeNode) {
        child.parent_id = Some(self.id);
        self.children.push(child);
    }
}

pub struct TreeWidget {
    pub root: TreeNode,
    selected_id: usize,
}

impl TreeWidget {
    pub fn new(root_text: String) -> Self {
        Self {
            root: TreeNode::new(0, root_text),
            selected_id: 0,
        }
    }
}

pub struct TreeWidgetRenderer<'a> {
    block: ratatui::widgets::Block<'a>,
    highlight_style: ratatui::style::Style,
}

impl<'a> TreeWidgetRenderer<'a> {
    pub fn new() -> Self {
        Self {
            block: ratatui::widgets::Block::default(),
            highlight_style: ratatui::style::Style::default(),
        }
    }

    pub fn block(mut self, block: ratatui::widgets::Block<'a>) -> Self {
        self.block = block;
        self
    }

    pub const fn selection_style(mut self, style: ratatui::style::Style) -> Self {
        self.highlight_style = style;
        self
    }

    fn render_node(&self, tree_area: Rect, buf: &mut Buffer, y: &mut u16, node: &TreeNode, state: &TreeWidget, lvl: usize) {
        let area = Rect::new(tree_area.x, *y, tree_area.width, 1);
        *y += 1;

        let style = if node.id == state.selected_id {
            self.highlight_style
        } else {
            ratatui::style::Style::default()
        };
        let node_text = format!(
            "{}{}{}",
            "│  ".repeat(lvl.saturating_sub(1)),
            "├──".repeat(if lvl == 0 { 0 } else { 1 }),
            node.text
        );
        Text::raw(node_text).style(style).render(area, buf);
        for child in &node.children {
            self.render_node(tree_area, buf, y, child, state, lvl + 1);
        }
    }
}

impl<'a> StatefulWidget for TreeWidgetRenderer<'a> {
    type State = TreeWidget;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let tree_area = self.block.inner(area);
        self.block.clone().render(area, buf);

        let mut y = tree_area.y;
        self.render_node(tree_area, buf, &mut y, &state.root, state, 0);
    }
}

#[test]
fn test_new_tree_widget() {
    let widget = TreeWidget::new("root".to_string());
    assert_eq!(widget.root.id, 0);
    assert_eq!(widget.root.text, "root".to_string());
    assert!(widget.root.parent_id.is_none());
    assert_eq!(widget.selected_id, widget.root.id);
}

#[test]
fn test_add_child() {
    let mut widget = TreeWidget::new("root".to_string());
    widget.root.add_child(TreeNode::new(1, "child".to_string()));
    assert_eq!(widget.root.children.len(), 1);
    assert_eq!(widget.root.children[0].id, 1);
    assert_eq!(widget.root.children[0].text, "child");
    assert_eq!(widget.root.children[0].parent_id, Some(0));
}
