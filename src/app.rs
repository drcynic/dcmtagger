use std::{io, path::Path};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::{self, border},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, StatefulWidget, Widget},
};
use tui_textarea::{Input, TextArea};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::dicom::DicomData;

#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Browse,
    Help,
    Cmd,
}

#[derive(Debug, Default)]
pub struct App<'a> {
    input_path: &'a str,
    dicom_data: DicomData,
    tree_items: Vec<TreeItem<'static, String>>,
    tree_state: TreeState<String>,
    text_area: TextArea<'a>,
    mode: Mode,
    page_size: usize,
    input_text: Option<String>,
    handler_text: String,
    exit: bool,
    help_scroll_offset: usize,
}

impl<'a> App<'a> {
    pub fn new(input_path: &'a str) -> anyhow::Result<Self> {
        let dicom_data = DicomData::new(Path::new(input_path))?;
        let root_item = dicom_data.tree_sorted_by_filename();
        let mut tree_state = TreeState::default();
        tree_state.select(vec![root_item.identifier().clone()]);
        tree_state.open(vec![root_item.identifier().clone()]);
        let mut text_area = TextArea::new(Vec::new());
        text_area.set_cursor_style(Style::default());

        Ok(App {
            input_path,
            dicom_data,
            tree_items: vec![root_item],
            tree_state,
            text_area,
            ..Default::default()
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let [list_area, _, _] = App::layouted_areas(frame.area());
        self.page_size = list_area.height.saturating_sub(4) as usize;

        frame.render_widget(self, frame.area());
    }

    fn layouted_areas(area: Rect) -> [Rect; 3] {
        Layout::vertical([Constraint::Fill(1), Constraint::Length(2), Constraint::Length(2)]).areas(area)
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // Add timeout to prevent blocking indefinitely
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self.handle_key_event(key_event),
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.mode {
            Mode::Browse => match key_event.code {
                KeyCode::Char('1') => self.sort_by_filename(),
                KeyCode::Char('2') => self.sort_by_tag(0),
                KeyCode::Char('3') => self.sort_by_tag(1),
                KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                KeyCode::Char('?') => self.show_help(),
                KeyCode::Char('/') => {
                    self.setup_input_edit('/');
                }
                KeyCode::Char(':') => {
                    self.setup_input_edit(':');
                }
                KeyCode::Up if key_event.modifiers.contains(KeyModifiers::SHIFT) => self.move_to_prev_sibling(),
                KeyCode::Char('K') => self.move_to_prev_sibling(),
                KeyCode::Down if key_event.modifiers.contains(KeyModifiers::SHIFT) => self.move_to_next_sibling(),
                KeyCode::Char('J') => self.move_to_next_sibling(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('n') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_down(),
                KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_half_page_down(),
                KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_half_page_up(),
                KeyCode::Char('f') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_page_down(),
                KeyCode::PageDown => self.move_page_down(),
                KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_page_up(),
                KeyCode::PageUp => self.move_page_up(),
                KeyCode::Char('g') => self.move_to_first(),
                KeyCode::Char('G') => self.move_to_last(),
                KeyCode::Char('0') | KeyCode::Char('^') => self.move_to_first_sibling(),
                KeyCode::Char('$') => self.move_to_last_sibling(),
                KeyCode::Char('c') => self.collapse_siblings(),
                KeyCode::Char('e') => self.expand_siblings(),
                KeyCode::Char('E') => self.expand_current_recursive(),
                KeyCode::Char('C') => self.collapse_current_recursive(),
                KeyCode::Enter | KeyCode::Char(' ') => self.toggle_node(),
                KeyCode::Char('H') => self.move_to_parent(),
                KeyCode::Left if key_event.modifiers.contains(KeyModifiers::SHIFT) => self.move_to_parent(),
                KeyCode::Char('L') => self.move_to_next_child(),
                KeyCode::Right if key_event.modifiers.contains(KeyModifiers::SHIFT) => self.move_to_next_child(),
                KeyCode::Right | KeyCode::Char('l') => self.move_into_tree(),
                KeyCode::Left | KeyCode::Char('h') => self.move_up_tree(),
                KeyCode::Char('n') => {
                    if let Some(text) = &self.input_text {
                        self.handler_text = format!("search for text: {}", text);
                    } else {
                        self.handler_text = "nothing to search for".to_string();
                    }
                }
                _ => {}
            },
            Mode::Cmd => match key_event.code {
                KeyCode::Esc => {
                    self.mode = Mode::Browse;
                    self.text_area.move_cursor(tui_textarea::CursorMove::Head);
                    self.text_area.delete_line_by_end();
                    self.text_area.set_cursor_style(Style::default());
                    self.input_text = None;
                }
                KeyCode::Enter => {
                    // self.mode = Mode::Browse;
                }
                _ => {
                    let input = Input::from(key_event);
                    if self.text_area.input(input) {
                        let current_text = &self.text_area.lines()[0];
                        self.input_text = if current_text.is_empty() {
                            self.mode = Mode::Browse;
                            self.text_area.set_cursor_style(Style::default());
                            None
                        } else {
                            Some(current_text.to_string())
                        }
                    }
                }
            },
            Mode::Help => match key_event.code {
                KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => self.hide_help(),
                KeyCode::Up | KeyCode::Char('k') => self.scroll_help_up(),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_help_down(),
                _ => {}
            },
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn show_help(&mut self) {
        self.mode = Mode::Help;
        self.help_scroll_offset = 0;
    }

    fn hide_help(&mut self) {
        self.mode = Mode::Browse;
    }

    fn scroll_help_up(&mut self) {
        if self.help_scroll_offset > 0 {
            self.help_scroll_offset -= 1;
        }
    }

    fn scroll_help_down(&mut self) {
        let help_lines = help_text().lines().collect::<Vec<&str>>();
        let max_scroll = help_lines.len().saturating_sub(3);
        if self.help_scroll_offset < max_scroll {
            self.help_scroll_offset += 1;
        }
    }

    fn setup_input_edit(&mut self, start_char: char) {
        self.mode = Mode::Cmd;
        self.text_area = TextArea::new(vec![start_char.to_string()]);
        self.text_area.move_cursor(tui_textarea::CursorMove::End);
        self.text_area.set_cursor_line_style(Style::default());
    }

    fn sort_by_filename(&mut self) {
        let root_item = self.dicom_data.tree_sorted_by_filename();
        self.tree_state = TreeState::default();
        self.tree_state.select(vec![root_item.identifier().clone()]);
        self.tree_state.open(vec![root_item.identifier().clone()]);
        self.tree_items = vec![root_item];
        self.handler_text = "sorted by filename".to_string();
    }

    fn sort_by_tag(&mut self, min_diffs: usize) {
        let root_item = self.dicom_data.tree_sorted_by_tag(min_diffs);
        self.tree_state = TreeState::default();
        self.tree_state.open(vec![root_item.identifier().clone()]);
        self.tree_state.select(vec![root_item.identifier().clone()]);
        // open all groups
        root_item.children().iter().for_each(|c| {
            self.tree_state.open(vec![root_item.identifier().clone(), c.identifier().clone()]);
        });
        self.tree_items = vec![root_item];
        if min_diffs == 0 {
            self.handler_text = "sorted by tag".to_string();
        } else {
            self.handler_text = "sorted by tag, displaying only different tags".to_string();
        }
    }

    fn move_down(&mut self) {
        self.handler_text = "down".to_string();
        self.tree_state.key_down();
    }

    fn move_up(&mut self) {
        self.handler_text = "up".to_string();
        self.tree_state.key_up();
    }

    fn move_half_page_down(&mut self) {
        self.handler_text = "ctrl + d -> half page down".to_string();
        self.tree_state
            .select_relative(|c| c.map_or(0, |c| c.saturating_add(self.page_size / 2)));
    }

    fn move_half_page_up(&mut self) {
        self.handler_text = "ctrl + u -> half page up".to_string();
        self.tree_state
            .select_relative(|c| c.map_or(0, |c| c.saturating_sub(self.page_size / 2)));
    }

    fn move_page_down(&mut self) {
        self.handler_text = "ctrl + f/page-down -> one screen down".to_string();
        self.tree_state
            .select_relative(|c| c.map_or(0, |c| c.saturating_add(self.page_size)));
    }

    fn move_page_up(&mut self) {
        self.handler_text = "ctrl + b/page-up -> one screen up".to_string();
        self.tree_state
            .select_relative(|c| c.map_or(0, |c| c.saturating_sub(self.page_size)));
    }

    fn move_to_first(&mut self) {
        self.handler_text = "g -> move to first".to_string();
        self.tree_state.select_first();
    }

    fn move_to_last(&mut self) {
        self.handler_text = "G -> move to last".to_string();
        self.tree_state.select_last();
    }

    fn toggle_node(&mut self) {
        self.handler_text = "toggled node".to_string();
        self.tree_state.toggle_selected();
    }

    fn expand_current_recursive(&mut self) {
        self.handler_text = "shift + E -> expand current node recursively".to_string();
        let selected = self.tree_state.selected().to_vec();
        self.handler_text = format!("selected: {:?}", &selected);
        if selected.is_empty() {
            return;
        }

        self.expand_node_recursive(selected);
    }

    fn collapse_current_recursive(&mut self) {
        self.handler_text = "shift + C -> collapse current node recursively".to_string();
        let selected = self.tree_state.selected().to_vec();
        self.handler_text = format!("selected: {:?}", &selected);

        if selected.len() == 1 {
            self.tree_state.close_all();
        } else if selected.len() > 1 {
            self.collapse_node_recursive(selected);
        }
    }

    fn expand_node_recursive(&mut self, node_path: Vec<String>) {
        let all_paths = self.collect_all_descendant_paths(node_path);
        for path in all_paths {
            self.tree_state.open(path);
        }
    }

    fn collapse_node_recursive(&mut self, node_path: Vec<String>) {
        let all_paths = self.collect_all_descendant_paths(node_path);
        for path in all_paths.into_iter().rev() {
            self.tree_state.close(&path);
        }
    }

    fn collect_all_descendant_paths(&self, node_path: Vec<String>) -> Vec<Vec<String>> {
        let mut all_paths = Vec::new();
        self.collect_paths_from_tree(&node_path, &mut all_paths);
        all_paths
    }

    fn collect_paths_from_tree(&self, target_path: &[String], all_paths: &mut Vec<Vec<String>>) {
        if target_path.len() == 1 {
            // Found the target item, collect it and all descendants
            let path = vec![self.tree_items[0].identifier().clone()];
            all_paths.push(path.clone());
            Self::collect_children_paths(&self.tree_items[0], &path, all_paths);
        } else {
            // Continue searching in children
            Self::collect_paths_from_children(
                self.tree_items[0].children(),
                &target_path[1..],
                &[target_path[0].clone()],
                all_paths,
            );
        }
    }

    fn collect_paths_from_children(
        children: &[TreeItem<String>],
        target_path: &[String],
        current_path: &[String],
        all_paths: &mut Vec<Vec<String>>,
    ) {
        for child in children {
            if child.identifier() == &target_path[0] {
                let mut child_path = current_path.to_vec();
                child_path.push(child.identifier().clone());

                if target_path.len() == 1 {
                    // Found the target, collect it and all descendants
                    all_paths.push(child_path.clone());
                    Self::collect_children_paths(child, &child_path, all_paths);
                } else {
                    // Continue searching deeper
                    Self::collect_paths_from_children(child.children(), &target_path[1..], &child_path, all_paths);
                }
                break;
            }
        }
    }

    fn collect_children_paths(item: &TreeItem<String>, item_path: &[String], all_paths: &mut Vec<Vec<String>>) {
        for child in item.children() {
            let mut child_path = item_path.to_vec();
            child_path.push(child.identifier().clone());
            all_paths.push(child_path.clone());
            Self::collect_children_paths(child, &child_path, all_paths);
        }
    }

    fn move_to_prev_sibling(&mut self) {
        self.handler_text = "K/shift+↑ -> move to previous sibling".to_string();
        self.move_to_sibling_by_direction(true);
    }

    fn move_to_next_sibling(&mut self) {
        self.handler_text = "J/shift+↓ -> move to next sibling".to_string();
        self.move_to_sibling_by_direction(false);
    }

    fn move_to_sibling_by_direction(&mut self, move_up: bool) {
        let selected = self.tree_state.selected();
        if selected.len() <= 1 {
            return;
        }

        // Find siblings at current level
        let parent_path = &selected[..selected.len() - 1];
        let current_id = &selected[selected.len() - 1];

        // Find parent item using tree traversal
        if let Some(siblings) = self.find_siblings_at_path(parent_path) {
            let current_idx = siblings.iter().position(|item| item.identifier() == current_id);

            if let Some(current_idx) = current_idx {
                let target_idx = if move_up {
                    if current_idx > 0 {
                        current_idx - 1
                    } else {
                        return;
                    }
                } else if current_idx + 1 < siblings.len() {
                    current_idx + 1
                } else {
                    return;
                };

                let mut target_path = parent_path.to_vec();
                target_path.push(siblings[target_idx].identifier().clone());
                self.tree_state.select(target_path);
            }
        }
    }

    fn find_siblings_at_path(&self, path: &[String]) -> Option<&[TreeItem<'_, String>]> {
        if path.is_empty() {
            return Some(self.tree_items.as_slice());
        }

        let mut current_items: &[TreeItem<String>] = self.tree_items.as_slice();

        for path_segment in path {
            if let Some(item) = current_items.iter().find(|item| item.identifier() == path_segment) {
                current_items = item.children();
            } else {
                return None;
            }
        }

        Some(current_items)
    }

    #[allow(dead_code)]
    fn open_all(&mut self) {
        self.handler_text = "shift + E -> expand all".to_string();
        let flat = self.tree_state.flatten(&self.tree_items);
        self.handler_text = format!("flat size: {}", flat.len());

        flat.iter().for_each(|node| {
            self.tree_state.open(node.identifier.clone());
        });
    }

    #[allow(dead_code)]
    fn close_all(&mut self) {
        self.handler_text = "shift + C -> collapse all".to_string();
        self.tree_state.close_all();
    }

    fn move_into_tree(&mut self) {
        self.handler_text = "l/→ -> move into tree".to_string();
        self.tree_state.key_right();
    }

    fn move_up_tree(&mut self) {
        self.handler_text = "h/← -> move up tree".to_string();
        self.tree_state.key_left();
    }

    fn move_to_parent(&mut self) {
        self.handler_text = "shift+H/shift+← -> move to parent".to_string();

        let selected = self.tree_state.selected();
        if selected.len() <= 1 {
            return; // Already at root or no selection
        }

        // Move to parent by removing the last element from the path
        self.tree_state.select(selected[..selected.len() - 1].to_vec());
    }

    fn move_to_next_child(&mut self) {
        self.handler_text = "shift+L/shift+→ -> move to next child".to_string();

        let selected = self.tree_state.selected();
        if selected.is_empty() {
            return;
        }

        let current = selected.to_vec();
        let flat_items = self.tree_state.flatten(&self.tree_items);

        // Find current item and if it has children
        if let Some(current_item) = flat_items.iter().find(|item| item.identifier == current)
            && !current_item.item.children().is_empty()
        {
            // If current node is collapsed and has children, expand it first
            if !self.tree_state.opened().contains(current.as_slice()) {
                self.tree_state.open(current.clone());
            }

            // Move to first child after expanding
            if let Some(first_child) = current_item.item.children().first() {
                let mut child_path = current;
                child_path.push(first_child.identifier().clone());
                self.tree_state.select(child_path);
            }
        }
    }

    fn move_to_first_sibling(&mut self) {
        self.handler_text = "0/^ -> move to first sibling".to_string();
        self.move_to_sibling(true);
    }

    fn move_to_last_sibling(&mut self) {
        self.handler_text = "$ -> move to last sibling".to_string();
        self.move_to_sibling(false);
    }

    fn move_to_sibling(&mut self, is_first: bool) {
        let selected = self.tree_state.selected();
        if selected.len() <= 1 {
            // At root level, move to first or last item
            if is_first {
                self.tree_state.select_first();
            } else {
                self.tree_state.select_last();
            }
            return;
        }

        // Get parent path
        let parent_path = &selected[..selected.len() - 1];
        let flat_items = self.tree_state.flatten(&self.tree_items);

        // Find parent item
        if let Some(parent_item) = flat_items.iter().find(|item| item.identifier == parent_path) {
            let target_child = if is_first {
                parent_item.item.children().first()
            } else {
                parent_item.item.children().last()
            };

            if let Some(target_child) = target_child {
                let mut target_sibling_path = parent_path.to_vec();
                target_sibling_path.push(target_child.identifier().clone());
                self.tree_state.select(target_sibling_path);
            }
        }
    }

    fn collapse_siblings(&mut self) {
        self.handler_text = "c -> collapse current node and siblings".to_string();
        self.apply_on_siblings(|tree_state, path| {
            if path.len() == 1 {
                tree_state.close(&[path[0].clone()]);
            } else {
                tree_state.close(&path);
            }
        });
    }

    fn expand_siblings(&mut self) {
        self.handler_text = "e -> expand current node and siblings".to_string();
        self.apply_on_siblings(|tree_state, path| {
            tree_state.open(path);
        });
    }

    fn apply_on_siblings<F>(&mut self, mut operation: F)
    where
        F: FnMut(&mut TreeState<String>, Vec<String>),
    {
        let selected = self.tree_state.selected();
        if selected.is_empty() {
            return;
        }

        if selected.len() == 1 {
            // At root level, operate on all root items
            for item in &self.tree_items {
                operation(&mut self.tree_state, vec![item.identifier().clone()]);
            }
        } else {
            // Find parent and operate on all its children (siblings)
            let parent_path = &selected[..selected.len() - 1];
            let flat_items = self.tree_state.flatten(&self.tree_items);

            if let Some(parent_item) = flat_items.iter().find(|item| item.identifier == parent_path) {
                let children_paths: Vec<_> = parent_item
                    .item
                    .children()
                    .iter()
                    .map(|child| {
                        let mut child_path = parent_path.to_vec();
                        child_path.push(child.identifier().clone());
                        child_path
                    })
                    .collect();

                for child_path in children_paths {
                    operation(&mut self.tree_state, child_path);
                }
            }
        }
    }

    fn render_help_overlay(&self, area: Rect, buf: &mut Buffer) {
        // Calculate centered popup area (roughly 60% width, 70% height)
        let popup_width = (area.width as f32 * 0.6) as u16;
        let popup_height = (area.height as f32 * 0.7) as u16;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + popup_x,
            y: area.y + popup_y,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        // Get help text lines and handle scrolling
        let help_lines = help_text().lines().collect::<Vec<&str>>();
        let visible_height = popup_height.saturating_sub(2) as usize; // Account for borders
        let start_line = self.help_scroll_offset;
        let end_line = (start_line + visible_height).min(help_lines.len());
        let help_text = help_lines[start_line..end_line].join("\n");

        let help_block = Block::bordered()
            .padding(Padding::horizontal(1))
            .title(Line::from(" DICOM Tagger Help".bold()).centered())
            .border_set(border::ROUNDED);

        Paragraph::new(help_text).block(help_block).render(popup_area, buf);
    }
}

impl<'a> Widget for &mut App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![" DICOM Tagger - ".bold(), self.input_path.into(), " ".into()]);
        // let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);

        let [list_area, state_area, input_area] = App::<'a>::layouted_areas(area);

        let bottom_vert_border_set = symbols::border::Set {
            bottom_left: symbols::line::NORMAL.vertical_right,
            bottom_right: symbols::line::NORMAL.vertical_left,
            ..symbols::border::PLAIN
        };

        let tree_block = Block::bordered()
            .title(title.centered())
            .border_set(bottom_vert_border_set)
            .padding(Padding::horizontal(1));
        let tree = Tree::new(&self.tree_items)
            .expect("all item identifiers are unique")
            .block(tree_block)
            .highlight_style(Style::default().bg(Color::DarkGray));
        StatefulWidget::render(tree, list_area, buf, &mut self.tree_state);

        let state_block = Block::bordered()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_set(bottom_vert_border_set);
        let handler_text = Text::from(vec![Line::from(vec!["Value: ".into(), self.handler_text.clone().yellow()])]);
        Paragraph::new(handler_text).centered().block(state_block).render(state_area, buf);

        let input_block = Block::bordered()
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_set(border::PLAIN);
        self.text_area.set_block(input_block);
        self.text_area.render(input_area, buf);

        // Render help overlay if shown
        if self.mode == Mode::Help {
            self.render_help_overlay(area, buf);
        }
    }
}

pub const fn help_text() -> &'static str {
    r#"Navigation:
  ?                    - Show help
  q/Esc                - Quit
  1                    - Sort tree by filename
  2                    - Sort tree by tags
  3                    - Sort tree by tags, only showing tags with different values
  k/↑/ctrl+p           - Move up
  j/↓/ctrl+n           - Move down
  h/←                  - Move to parent or close node
  l/→                  - Expand node or move to first child
  H/shift+←            - Move to parent
  L/shift+→            - Move to next child (expand if collapsed)
  J/shift+↓            - Move to next sibling (same level)
  K/shift+↑            - Move to previous sibling (same level)
  ctrl+u               - Move half page up
  ctrl+d               - Move half page down
  ctrl+f/page-down     - Move page down
  ctrl+b/page-up       - Move page up
  g                    - Move to first element
  G                    - Move to last element
  0/^                  - Move to first sibling
  $                    - Move to last sibling
  Enter/Space          - Toggle expand/collapse
  c                    - Collapse current node and siblings
  e                    - Expand current node and siblings
  E                    - Expand current node recursively
  C                    - Collapse current node recursively
"#
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::collections::BTreeMap;

//     #[test]
//     fn test_file_tree_structure() {
// let dict = dicom_dictionary_std::StandardDataDictionary;
// let mut tags = BTreeMap::new();
// let elements = vec![];
// tags.insert(0x0008, elements.clone());
// tags.insert(0x0010, elements);
// let di = DicomInput {
//     base_path: "root".to_string(),
//     file_tags: BTreeMap::from([("01.dcm".to_string(), tags)]),
// };

// let root_item = build_tree(&di);

// println!("root: {:?}", &root_item);
// let children = root_item.children();
// println!("Children: {:?}", &children);
// assert_eq!(children.len(), 2, "Should have 2 groups");

// assert_eq!(children[0].identifier(), "group_0008");
// assert_eq!(children[1].identifier(), "group_0010");
// }
// }
