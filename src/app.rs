use std::fmt::Debug;
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

use crate::dicom::DicomData;
use crate::tree_widget;

#[derive(Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Browse,
    Help,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchDirection {
    Forward,
    _Backward,
}

#[derive(Debug, Default)]
pub struct App<'a> {
    input_path: &'a str,
    dicom_data: DicomData,
    tree_widget: tree_widget::TreeWidget,
    text_area: TextArea<'a>,
    mode: Mode,
    page_size: usize,
    input_text: Option<String>,
    search_start_node_id: Vec<usize>,
    handler_text: String,
    exit: bool,
    help_scroll_offset: usize,
}

impl<'a> App<'a> {
    pub fn new(input_path: &'a str) -> anyhow::Result<Self> {
        let dicom_data = DicomData::new(Path::new(input_path))?;
        let mut text_area = TextArea::new(Vec::new());
        text_area.set_cursor_style(Style::default());

        let mut tree_widget = dicom_data.tree_sorted_by_filename();
        tree_widget.open(tree_widget.root_id);

        Ok(App {
            input_path,
            dicom_data,
            tree_widget,
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
        self.page_size = list_area.height.saturating_sub(2) as usize;

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
                KeyCode::Char('N') => {
                    // let start_node = self.tree_state.selected().to_vec();
                    // self.try_search(SearchDirection::Backward, &start_node);
                }
                KeyCode::Char('n') => {
                    // let start_node = self.tree_state.selected().to_vec();
                    // self.try_search(SearchDirection::Forward, &start_node);
                }
                _ => {}
            },
            Mode::Search => match key_event.code {
                KeyCode::Esc => {
                    self.mode = Mode::Browse;
                    self.text_area.move_cursor(tui_textarea::CursorMove::Head);
                    self.text_area.delete_line_by_end();
                    self.text_area.set_cursor_style(Style::default());
                    self.input_text = None;
                }
                KeyCode::Enter => {
                    self.mode = Mode::Browse;
                    self.text_area.set_cursor_style(Style::default());
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
                        };
                        self.try_search(SearchDirection::Forward, &self.search_start_node_id.to_vec());
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
        self.mode = Mode::Search;
        // self.search_start_node_id = self.tree_state.selected().to_vec();
        let start_text = vec![if let Some(text) = &self.input_text {
            text.to_string()
        } else {
            start_char.to_string()
        }];
        self.text_area = TextArea::new(start_text);
        self.text_area.move_cursor(tui_textarea::CursorMove::End);
        self.text_area.set_cursor_line_style(Style::default());
    }

    fn sort_by_filename(&mut self) {
        self.tree_widget = self.dicom_data.tree_sorted_by_filename();
        self.tree_widget.open(self.tree_widget.root_id);
        self.handler_text = "sorted by filename".to_string();
    }

    fn sort_by_tag(&mut self, min_diff: usize) {
        self.tree_widget = self.dicom_data.tree_sorted_by_tag(min_diff);
        self.tree_widget.open(self.tree_widget.root_id);
        let root_node = self.tree_widget.nodes.get(self.tree_widget.root_id).unwrap();
        let children = root_node.children.clone();
        for child_id in children {
            self.tree_widget.open(child_id);
        }

        if min_diff == 0 {
            self.handler_text = "sorted by tag".to_string();
        } else {
            self.handler_text = "sorted by tag, displaying only different tags".to_string();
        }
    }

    fn move_down(&mut self) {
        self.handler_text = "down".to_string();
        self.tree_widget.select_next(1);
    }

    fn move_up(&mut self) {
        self.handler_text = "up".to_string();
        self.tree_widget.select_prev(1);
    }

    fn move_half_page_down(&mut self) {
        self.handler_text = "ctrl + d -> half page down".to_string();
        self.tree_widget.select_next(self.page_size / 2);
    }

    fn move_half_page_up(&mut self) {
        self.handler_text = "ctrl + u -> half page up".to_string();
        self.tree_widget.select_prev(self.page_size / 2);
    }

    fn move_page_down(&mut self) {
        self.handler_text = "ctrl + f/page-down -> one screen down".to_string();
        self.tree_widget.select_next(self.page_size);
    }

    fn move_page_up(&mut self) {
        self.handler_text = "ctrl + b/page-up -> one screen up".to_string();
        self.tree_widget.select_prev(self.page_size);
    }

    fn move_to_first(&mut self) {
        self.handler_text = "g -> move to first".to_string();
        self.tree_widget.selected_id = self.tree_widget.root_id;
    }

    fn move_to_last(&mut self) {
        self.handler_text = "G -> move to last".to_string();
        let vn = self.tree_widget.visible_nodes();
        self.tree_widget.selected_id = *vn.last().unwrap();
    }

    fn toggle_node(&mut self) {
        self.handler_text = "toggled node".to_string();
        self.tree_widget.toggle_selected();
    }

    fn expand_current_recursive(&mut self) {
        self.handler_text = "shift + E -> expand current node recursively".to_string();
        todo!()
    }

    fn collapse_current_recursive(&mut self) {
        self.handler_text = "shift + C -> collapse current node recursively".to_string();
        todo!()
    }

    fn move_to_prev_sibling(&mut self) {
        self.handler_text = "K/shift+↑ -> move to previous sibling".to_string();
        self.tree_widget.select_prev_sibling();
    }

    fn move_to_next_sibling(&mut self) {
        self.handler_text = "J/shift+↓ -> move to next sibling".to_string();
        self.tree_widget.select_next_sibling();
    }

    #[allow(dead_code)]
    fn open_all(&mut self) {
        self.handler_text = "shift + E -> expand all".to_string();
        todo!()
    }

    #[allow(dead_code)]
    fn close_all(&mut self) {
        self.handler_text = "shift + C -> collapse all".to_string();
        todo!()
    }

    fn move_into_tree(&mut self) {
        self.handler_text = "l/→ -> move into tree".to_string();
        todo!()
    }

    fn move_up_tree(&mut self) {
        self.handler_text = "h/← -> move up tree".to_string();
        todo!()
    }

    fn move_to_parent(&mut self) {
        self.handler_text = "shift+H/shift+← -> move to parent".to_string();
        todo!()
    }

    fn move_to_next_child(&mut self) {
        self.handler_text = "shift+L/shift+→ -> move to next child".to_string();
        todo!()
    }

    fn move_to_first_sibling(&mut self) {
        self.handler_text = "0/^ -> move to first sibling".to_string();
        todo!()
    }

    fn move_to_last_sibling(&mut self) {
        self.handler_text = "$ -> move to last sibling".to_string();
        todo!()
    }

    fn collapse_siblings(&mut self) {
        self.handler_text = "c -> collapse current node and siblings".to_string();
        todo!()
    }

    fn expand_siblings(&mut self) {
        self.handler_text = "e -> expand current node and siblings".to_string();
        todo!()
    }

    fn try_search(&mut self, _dir: SearchDirection, _start_node: &[usize]) {
        if let Some(_text) = &self.input_text {
            todo!()
        } else {
            self.handler_text = "nothing to search for".to_string();
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
            .padding(Padding::horizontal(0));
        // let tree = Tree::new(&self.tree_items)
        //     .expect("all item identifiers are unique")
        //     .block(tree_block)
        //     .highlight_style(Style::default().bg(Color::DarkGray));
        // StatefulWidget::render(tree, list_area, buf, &mut self.tree_state);

        // !todo: check if this is fast enough for very large tree with > 150k nodes all opened
        let visible = self.tree_widget.visible_nodes();
        let start_idx = visible.iter().position(|&id| id == self.tree_widget.visible_start_id).unwrap();
        let sel_idx = visible.iter().position(|&id| id == self.tree_widget.selected_id).unwrap();
        if sel_idx < start_idx {
            self.tree_widget.visible_start_id = self.tree_widget.selected_id;
        } else if sel_idx - start_idx >= self.page_size {
            // selection not visible, move start to ensure
            self.tree_widget.visible_start_id = visible[sel_idx.saturating_sub(self.page_size - 1)];
        }

        let tree_renderer = tree_widget::TreeWidgetRenderer::new()
            .block(tree_block)
            .selection_style(Style::default().bg(Color::DarkGray));
        StatefulWidget::render(tree_renderer, list_area, buf, &mut self.tree_widget);

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
