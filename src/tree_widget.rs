use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{StatefulWidget, Widget},
};
use slotmap::SlotMap;

pub struct TreeNode {
    text: String,
    children: Vec<slotmap::DefaultKey>,
    parent_id: Option<slotmap::DefaultKey>,
}

impl TreeNode {
    pub fn new(text: String) -> Self {
        Self {
            text,
            children: Vec::new(),
            parent_id: None,
        }
    }
}

pub struct TreeWidget {
    pub root_id: slotmap::DefaultKey,
    pub selected_id: slotmap::DefaultKey,
    pub open_nodes: HashSet<slotmap::DefaultKey>,
    pub nodes: SlotMap<slotmap::DefaultKey, TreeNode>,
}

impl TreeWidget {
    pub fn new(root_text: String) -> Self {
        let mut nodes = SlotMap::new();
        let root_id = nodes.insert(TreeNode::new(root_text));
        Self {
            root_id,
            selected_id: root_id,
            open_nodes: HashSet::new(),
            nodes,
        }
    }

    pub fn add_child(&mut self, text: &str, parent_id: slotmap::DefaultKey) -> slotmap::DefaultKey {
        let mut child = TreeNode::new(text.to_string());
        child.parent_id = Some(parent_id);
        let child_id = self.nodes.insert(child);
        let parent = self.nodes.get_mut(parent_id).unwrap();
        parent.children.push(child_id);
        child_id
    }

    #[allow(dead_code)]
    pub fn is_open(&self, node_id: &slotmap::DefaultKey) -> bool {
        self.open_nodes.contains(node_id)
    }

    #[allow(dead_code)]
    pub fn toggle(&mut self, node_id: slotmap::DefaultKey) {
        if self.open_nodes.contains(&node_id) {
            self.open_nodes.remove(&node_id);
        } else {
            self.open_nodes.insert(node_id);
        }
    }

    #[allow(dead_code)]
    pub fn open(&mut self, node_id: slotmap::DefaultKey) {
        if !self.open_nodes.contains(&node_id) {
            self.open_nodes.insert(node_id);
        }
    }

    #[allow(dead_code)]
    pub fn close(&mut self, node_id: slotmap::DefaultKey) {
        if self.open_nodes.contains(&node_id) {
            self.open_nodes.remove(&node_id);
        }
    }

    #[allow(dead_code)]
    pub fn select_next(&mut self) {
        if let Some(next_id) = self.next(self.selected_id) {
            self.selected_id = next_id;
        }
    }

    #[allow(dead_code)]
    pub fn select_prev(&mut self) {
        if let Some(next_id) = self.prev(self.selected_id) {
            self.selected_id = next_id;
        }
    }

    fn next(&self, cur_id: slotmap::DefaultKey) -> Option<slotmap::DefaultKey> {
        let cur = self.nodes.get(cur_id).unwrap();
        if !cur.children.is_empty() && self.open_nodes.contains(&self.selected_id) {
            Some(cur.children[0])
        } else {
            let mut cur_id = self.selected_id;
            let mut parent_id_opt = cur.parent_id;
            loop {
                if let Some(parent_id) = parent_id_opt {
                    if let Some(sibling_id) = self.next_sibling(parent_id, cur_id) {
                        return Some(sibling_id);
                    }
                    cur_id = parent_id;
                    parent_id_opt = self.nodes.get(parent_id).unwrap().parent_id;
                } else {
                    return None;
                }
            }
        }
    }

    fn prev(&self, cur_id: slotmap::DefaultKey) -> Option<slotmap::DefaultKey> {
        let cur = self.nodes.get(cur_id).unwrap();

        let parent_id = cur.parent_id?;
        if let Some(sibling_id) = self.prev_sibling(parent_id, cur_id) {
            let mut cur_id = sibling_id;
            loop {
                let cur = self.nodes.get(cur_id).unwrap();
                if !cur.children.is_empty() && self.open_nodes.contains(&cur_id) {
                    cur_id = *cur.children.last().unwrap();
                } else {
                    return Some(cur_id);
                }
            }
        } else {
            Some(parent_id)
        }
    }

    fn next_sibling(&self, parent_id: slotmap::DefaultKey, cur_id: slotmap::DefaultKey) -> Option<slotmap::DefaultKey> {
        let parent = self.nodes.get(parent_id).unwrap();
        let index = parent.children.iter().position(|&id| id == cur_id).unwrap();
        if index + 1 < parent.children.len() {
            Some(parent.children[index + 1])
        } else {
            None
        }
    }

    fn prev_sibling(&self, parent_id: slotmap::DefaultKey, cur_id: slotmap::DefaultKey) -> Option<slotmap::DefaultKey> {
        let parent = self.nodes.get(parent_id).unwrap();
        let index = parent.children.iter().position(|&id| id == cur_id).unwrap();
        if index > 0 { Some(parent.children[index - 1]) } else { None }
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

    fn render_node(&self, tree_area: Rect, buf: &mut Buffer, y: &mut u16, node_id: slotmap::DefaultKey, state: &TreeWidget, lvl: usize) {
        let area = Rect::new(tree_area.x, *y, tree_area.width, 1);
        *y += 1;

        let style = if node_id == state.selected_id {
            self.highlight_style
        } else {
            ratatui::style::Style::default()
        };
        let node = state.nodes.get(node_id).unwrap();
        let node_text = format!(
            "{}{}{}",
            "│  ".repeat(lvl.saturating_sub(1)),
            "├──".repeat(if lvl == 0 { 0 } else { 1 }),
            node.text
        );
        Text::raw(node_text).style(style).render(area, buf);
        if state.open_nodes.contains(&node_id) {
            for child_id in &node.children {
                self.render_node(tree_area, buf, y, *child_id, state, lvl + 1);
            }
        }
    }
}

impl<'a> StatefulWidget for TreeWidgetRenderer<'a> {
    type State = TreeWidget;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let tree_area = self.block.inner(area);
        self.block.clone().render(area, buf);

        let mut y = tree_area.y;
        self.render_node(tree_area, buf, &mut y, state.root_id, state, 0);
    }
}

#[test]
fn test_new_tree_widget() {
    let tree_widget = TreeWidget::new("root".to_string());
    let root_node = tree_widget.nodes.get(tree_widget.root_id).unwrap();
    assert_eq!(root_node.text, "root".to_string());
    assert!(root_node.parent_id.is_none());
    assert_eq!(tree_widget.selected_id, tree_widget.root_id);
    assert!(tree_widget.open_nodes.is_empty());
}

#[test]
fn test_add_child() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child_id = tree_widget.add_child("child", tree_widget.root_id);
    let root_node = tree_widget.nodes.get(tree_widget.root_id).unwrap();
    let child_node = tree_widget.nodes.get(child_id).unwrap();
    assert_eq!(root_node.children.len(), 1);
    assert_eq!(root_node.children[0], child_id);
    assert_eq!(child_node.text, "child");
    assert_eq!(child_node.parent_id, Some(tree_widget.root_id));
}

#[test]
fn test_toggle_root() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    tree_widget.add_child("child", tree_widget.root_id);
    assert!(!tree_widget.is_open(&tree_widget.root_id));

    tree_widget.toggle(tree_widget.root_id);
    assert!(tree_widget.is_open(&tree_widget.root_id));
}

#[test]
fn test_toggle_child() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child1_id = tree_widget.add_child("child1", tree_widget.root_id);
    tree_widget.add_child("child2", child1_id);
    tree_widget.add_child("child3", child1_id);
    tree_widget.add_child("child4", tree_widget.root_id);
    tree_widget.add_child("child5", tree_widget.root_id);
    assert!(!tree_widget.is_open(&child1_id));

    tree_widget.toggle(tree_widget.root_id);
    tree_widget.toggle(child1_id);
    assert!(tree_widget.is_open(&child1_id));
}

#[test]
fn test_next_on_closed_root() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    tree_widget.add_child("child1", tree_widget.root_id);

    assert_eq!(tree_widget.selected_id, tree_widget.root_id);
    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, tree_widget.root_id);
}

#[test]
fn test_next_on_open_root() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child_id = tree_widget.add_child("child", tree_widget.root_id);

    assert_eq!(tree_widget.selected_id, tree_widget.root_id);
    tree_widget.open(tree_widget.root_id);
    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child_id);
}

#[test]
fn test_next_for_closed_children() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child1_id = tree_widget.add_child("child1", tree_widget.root_id);
    tree_widget.add_child("child2", child1_id);
    let child3_id = tree_widget.add_child("child3", child1_id);
    tree_widget.add_child("child4", child3_id);
    let child5_id = tree_widget.add_child("child5", tree_widget.root_id);
    let child6_id = tree_widget.add_child("child6", tree_widget.root_id);
    tree_widget.open(tree_widget.root_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child1_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child5_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child6_id);
}

#[test]
fn test_next_for_opented_children() {
    // root
    // ├─child1
    // │ ├─child2
    // │ ├─child3
    // │   ├─child4
    // ├─child5
    // ├─child6
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child1_id = tree_widget.add_child("child1", tree_widget.root_id);
    let child2_id = tree_widget.add_child("child2", child1_id);
    let child3_id = tree_widget.add_child("child3", child1_id);
    let child4_id = tree_widget.add_child("child4", child3_id);
    let child5_id = tree_widget.add_child("child5", tree_widget.root_id);
    let child6_id = tree_widget.add_child("child6", tree_widget.root_id);
    tree_widget.open(tree_widget.root_id);
    tree_widget.open(child1_id);
    tree_widget.open(child3_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child1_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child2_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child3_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child4_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child5_id);

    tree_widget.select_next();
    assert_eq!(tree_widget.selected_id, child6_id);
}

#[test]
fn test_prev_for_closed_children() {
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child1_id = tree_widget.add_child("child1", tree_widget.root_id);
    tree_widget.add_child("child2", child1_id);
    let child3_id = tree_widget.add_child("child3", child1_id);
    tree_widget.add_child("child4", child3_id);
    let child5_id = tree_widget.add_child("child5", tree_widget.root_id);
    let child6_id = tree_widget.add_child("child6", tree_widget.root_id);
    tree_widget.selected_id = child6_id;
    tree_widget.open(tree_widget.root_id);
    assert_eq!(tree_widget.selected_id, child6_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child5_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child1_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, tree_widget.root_id);
}

#[test]
fn test_prev_for_opented_children() {
    // root
    // ├─child1
    // │ ├─child2
    // │ ├─child3
    // │   ├─child4
    // ├─child5
    // ├─child6
    let mut tree_widget = TreeWidget::new("root".to_string());
    let child1_id = tree_widget.add_child("child1", tree_widget.root_id);
    let child2_id = tree_widget.add_child("child2", child1_id);
    let child3_id = tree_widget.add_child("child3", child1_id);
    let child4_id = tree_widget.add_child("child4", child3_id);
    let child5_id = tree_widget.add_child("child5", tree_widget.root_id);
    let child6_id = tree_widget.add_child("child6", tree_widget.root_id);
    tree_widget.open(tree_widget.root_id);
    tree_widget.open(child1_id);
    tree_widget.open(child3_id);
    tree_widget.selected_id = child6_id;

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child5_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child4_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child3_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child2_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, child1_id);

    tree_widget.select_prev();
    assert_eq!(tree_widget.selected_id, tree_widget.root_id);
}
