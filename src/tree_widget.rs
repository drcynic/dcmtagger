use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{StatefulWidget, Widget},
};
use slotmap::SlotMap;

pub type Id = slotmap::DefaultKey;

#[derive(Debug, Default)]
pub struct TreeNode {
    pub text: String,
    pub children: Vec<Id>,
    pub parent_id: Option<Id>,
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

#[derive(Debug, Default)]
pub struct TreeWidget {
    pub root_id: Id,
    pub visible_start_id: Id,
    pub selected_id: Id,
    pub open_nodes: HashSet<Id>,
    pub nodes: SlotMap<Id, TreeNode>,
}

impl TreeWidget {
    pub fn new(root_text: String) -> Self {
        let mut nodes = SlotMap::new();
        let root_id = nodes.insert(TreeNode::new(root_text));
        Self {
            root_id,
            visible_start_id: root_id,
            selected_id: root_id,
            open_nodes: HashSet::new(),
            nodes,
        }
    }

    pub fn add_child(&mut self, text: &str, parent_id: Id) -> Id {
        let mut child = TreeNode::new(text.to_string());
        child.parent_id = Some(parent_id);
        let child_id = self.nodes.insert(child);
        let parent = self.nodes.get_mut(parent_id).unwrap();
        parent.children.push(child_id);
        child_id
    }

    #[allow(dead_code)]
    pub fn is_open(&self, node_id: &Id) -> bool {
        self.open_nodes.contains(node_id)
    }

    pub fn toggle_selected(&mut self) {
        self.toggle(self.selected_id);
    }

    pub fn toggle(&mut self, node_id: Id) {
        if self.open_nodes.contains(&node_id) {
            self.open_nodes.remove(&node_id);
        } else {
            self.open_nodes.insert(node_id);
        }
    }

    pub fn open(&mut self, node_id: Id) {
        self.open_nodes.insert(node_id);
        // climb up hierarchy and open all parents
        let mut node_id = node_id;
        while let Some(node) = self.nodes.get(node_id)
            && let Some(parent_id) = node.parent_id
        {
            self.open_nodes.insert(node_id);
            node_id = parent_id;
        }
    }

    pub fn close(&mut self, node_id: Id) {
        self.open_nodes.remove(&node_id);
    }

    pub fn select_next(&mut self, offset: usize) {
        for _ in 0..offset {
            if let Some(next_id) = self.next_visible(self.selected_id) {
                self.selected_id = next_id;
            } else {
                break;
            }
        }
    }

    pub fn select_prev(&mut self, offset: usize) {
        for _ in 0..offset {
            if let Some(next_id) = self.prev_visible(self.selected_id) {
                self.selected_id = next_id;
            } else {
                break;
            }
        }
    }

    pub fn visible_nodes(&self) -> Vec<Id> {
        let mut v = Vec::new();
        self.gen_visible_nodes_recursive(&mut v, self.root_id);
        v
    }

    fn gen_visible_nodes_recursive(&self, v: &mut Vec<Id>, id: Id) {
        v.push(id);
        if let Some(node) = self.nodes.get(id)
            && self.open_nodes.contains(&id)
        {
            for child_id in &node.children {
                self.gen_visible_nodes_recursive(v, *child_id);
            }
        }
    }

    pub fn next(&self, cur_id: Id, only_opened: bool) -> Option<Id> {
        let cur = self.nodes.get(cur_id).unwrap();
        if !cur.children.is_empty() && (self.open_nodes.contains(&cur_id) || !only_opened) {
            Some(cur.children[0])
        } else {
            let mut cur_id = cur_id;
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

    pub fn next_visible(&self, cur_id: Id) -> Option<Id> {
        self.next(cur_id, true)
    }

    pub fn prev(&self, cur_id: Id, only_opened: bool) -> Option<Id> {
        let cur = self.nodes.get(cur_id).unwrap();

        let parent_id = cur.parent_id?;
        if let Some(sibling_id) = self.prev_sibling(parent_id, cur_id) {
            let mut cur_id = sibling_id;
            loop {
                let cur = self.nodes.get(cur_id).unwrap();
                if !cur.children.is_empty() && (self.open_nodes.contains(&cur_id) || !only_opened) {
                    cur_id = *cur.children.last().unwrap();
                } else {
                    return Some(cur_id);
                }
            }
        } else {
            Some(parent_id)
        }
    }

    pub fn prev_visible(&self, cur_id: Id) -> Option<Id> {
        self.prev(cur_id, true)
    }

    pub fn select_next_sibling(&mut self) {
        let sel_node = self.nodes.get(self.selected_id).unwrap();
        if let Some(parent_id) = sel_node.parent_id
            && let Some(sibling) = self.next_sibling(parent_id, self.selected_id)
        {
            self.selected_id = sibling;
        }
    }

    pub fn select_prev_sibling(&mut self) {
        let sel_node = self.nodes.get(self.selected_id).unwrap();
        if let Some(parent_id) = sel_node.parent_id
            && let Some(sibling) = self.prev_sibling(parent_id, self.selected_id)
        {
            self.selected_id = sibling;
        }
    }

    fn next_sibling(&self, parent_id: Id, cur_id: Id) -> Option<Id> {
        let parent = self.nodes.get(parent_id).unwrap();
        let index = parent.children.iter().position(|&id| id == cur_id)?;
        if index + 1 < parent.children.len() {
            Some(parent.children[index + 1])
        } else {
            None
        }
    }

    fn prev_sibling(&self, parent_id: Id, cur_id: Id) -> Option<Id> {
        let parent = self.nodes.get(parent_id).unwrap();
        let index = parent.children.iter().position(|&id| id == cur_id).unwrap();
        if index > 0 { Some(parent.children[index - 1]) } else { None }
    }

    pub fn level(&self, node_id: Id) -> usize {
        let mut node = self.nodes.get(node_id).unwrap();
        let mut level = 0;
        while let Some(parent_id) = node.parent_id {
            level += 1;
            node = self.nodes.get(parent_id).unwrap();
        }
        level
    }

    pub fn expand_recursive(&mut self, id: Id) {
        if let Some(cur) = self.nodes.get(id)
            && !cur.children.is_empty()
        {
            self.open_nodes.insert(id);
            let children = cur.children.clone(); // otherwise borrowing conflict with &mut self
            for child_id in children {
                self.expand_recursive(child_id);
            }
        }
    }

    pub fn collapse_recursive(&mut self, id: Id) {
        if let Some(cur) = self.nodes.get(id)
            && !cur.children.is_empty()
        {
            self.open_nodes.remove(&id);
            let children = cur.children.clone(); // otherwise borrowing conflict with &mut self
            for child_id in children {
                self.collapse_recursive(child_id);
            }
        }
    }

    pub fn siblings(&self, key: Id) -> Vec<Id> {
        if let Some(parent_id) = self.nodes.get(key).and_then(|node| node.parent_id) {
            self.nodes.get(parent_id).map_or(vec![], |parent| parent.children.clone())
        } else {
            vec![]
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

    fn render_node(&self, area: Rect, buf: &mut Buffer, node_id: Id, state: &TreeWidget, lvl: usize) {
        let style = if node_id == state.selected_id {
            self.highlight_style
        } else {
            ratatui::style::Style::default()
        };
        let node = state.nodes.get(node_id).unwrap();
        let node_text = format!(
            "{}{}{}{}",
            "│  ".repeat(lvl.saturating_sub(1)),
            if lvl == 0 { "" } else { "├──" },
            node.text,
            if !node.children.is_empty() { "/" } else { "" }
        );
        Text::raw(node_text).style(style).render(area, buf);
    }
}

impl<'a> StatefulWidget for TreeWidgetRenderer<'a> {
    type State = TreeWidget;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let tree_area = self.block.inner(area);
        self.block.clone().render(area, buf);

        let mut node_id = state.visible_start_id;
        for y in tree_area.y..tree_area.y + tree_area.height {
            let area = Rect::new(tree_area.x, y, tree_area.width, 1);
            self.render_node(area, buf, node_id, state, state.level(node_id));

            if let Some(next_id) = state.next_visible(node_id) {
                node_id = next_id;
            } else {
                break; // nothing more to draw -> break
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, tree_widget.root_id);
    }

    #[test]
    fn test_next_on_open_root() {
        let mut tree_widget = TreeWidget::new("root".to_string());
        let child_id = tree_widget.add_child("child", tree_widget.root_id);

        assert_eq!(tree_widget.selected_id, tree_widget.root_id);
        tree_widget.open(tree_widget.root_id);
        tree_widget.select_next(1);
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

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child1_id);

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child5_id);

        tree_widget.select_next(1);
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

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child1_id);

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child2_id);

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child3_id);

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child4_id);

        tree_widget.select_next(1);
        assert_eq!(tree_widget.selected_id, child5_id);

        tree_widget.select_next(1);
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

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child5_id);

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child1_id);

        tree_widget.select_prev(1);
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

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child5_id);

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child4_id);

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child3_id);

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child2_id);

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, child1_id);

        tree_widget.select_prev(1);
        assert_eq!(tree_widget.selected_id, tree_widget.root_id);
    }

    #[test]
    fn test_level() {
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

        assert_eq!(tree_widget.level(tree_widget.root_id), 0);
        assert_eq!(tree_widget.level(child1_id), 1);
        assert_eq!(tree_widget.level(child2_id), 2);
        assert_eq!(tree_widget.level(child3_id), 2);
        assert_eq!(tree_widget.level(child4_id), 3);
        assert_eq!(tree_widget.level(child5_id), 1);
        assert_eq!(tree_widget.level(child6_id), 1);
    }

    #[test]
    fn test_visible_nodes_with_idx_closed() {
        let mut tree_widget = TreeWidget::new("root".to_string());
        let child1_id = tree_widget.add_child("child1", tree_widget.root_id);
        tree_widget.add_child("child2", child1_id);
        let child3_id = tree_widget.add_child("child3", child1_id);
        tree_widget.add_child("child4", child3_id);
        let child5_id = tree_widget.add_child("child5", tree_widget.root_id);
        let child6_id = tree_widget.add_child("child6", tree_widget.root_id);
        tree_widget.open(tree_widget.root_id);

        let vni = tree_widget.visible_nodes();
        assert_eq!(vni.len(), 4);
        assert_eq!(vni.iter().position(|&id| id == tree_widget.root_id).unwrap(), 0);
        assert_eq!(vni.iter().position(|&id| id == child1_id).unwrap(), 1);
        assert_eq!(vni.iter().position(|&id| id == child5_id).unwrap(), 2);
        assert_eq!(vni.iter().position(|&id| id == child6_id).unwrap(), 3);
    }

    #[test]
    fn test_visible_nodes_with_idx_all_open() {
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

        let vni = tree_widget.visible_nodes();
        assert_eq!(vni.len(), 7);
        assert_eq!(vni.iter().position(|&id| id == tree_widget.root_id).unwrap(), 0);
        assert_eq!(vni.iter().position(|&id| id == child1_id).unwrap(), 1);
        assert_eq!(vni.iter().position(|&id| id == child2_id).unwrap(), 2);
        assert_eq!(vni.iter().position(|&id| id == child3_id).unwrap(), 3);
        assert_eq!(vni.iter().position(|&id| id == child4_id).unwrap(), 4);
        assert_eq!(vni.iter().position(|&id| id == child5_id).unwrap(), 5);
        assert_eq!(vni.iter().position(|&id| id == child6_id).unwrap(), 6);
    }
}
