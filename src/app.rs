use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::{self, border},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Padding, Paragraph, StatefulWidget, Widget},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::dicom::{GroupedTags, grouped_tags};

#[derive(Debug, Default)]
pub struct App<'a> {
    input_file: &'a str,
    tags: GroupedTags,
    tree_items: Vec<TreeItem<'static, String>>,
    tree_state: TreeState<String>,
    handler_text: String,
    exit: bool,
    show_help: bool,
    help_scroll_offset: usize,
}

impl<'a> App<'a> {
    pub fn new(input_file: &'a str) -> anyhow::Result<Self> {
        let tags = grouped_tags(&input_file)?;
        let tree_items = build_tree_items(&tags);
        let mut tree_state = TreeState::default();
        tree_state.select_first();

        Ok(App {
            input_file,
            tags,
            tree_items,
            tree_state,
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

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self.handle_key_event(key_event),
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.show_help {
            // When help is shown, handle keys differently
            match key_event.code {
                KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => self.hide_help(),
                KeyCode::Up | KeyCode::Char('k') => self.scroll_help_up(),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_help_down(),
                _ => {}
            }
        } else {
            match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.exit(),
                KeyCode::Char('?') => self.show_help(),
                KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_up(),
                KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('n') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_down(),
                KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_half_page_down(),
                KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.move_half_page_up(),
                KeyCode::Char('g') => self.move_to_first(),
                KeyCode::Char('G') => self.move_to_last(),
                KeyCode::Char('E') => self.open_all(),
                KeyCode::Char('C') => self.close_all(),
                KeyCode::Enter | KeyCode::Char(' ') => self.toggle_node(),
                _ => {}
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn show_help(&mut self) {
        self.show_help = true;
        self.help_scroll_offset = 0;
    }

    fn hide_help(&mut self) {
        self.show_help = false;
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
        self.tree_state.select_relative(|c| c.map_or(0, |c| c.saturating_add(10)));
    }

    fn move_half_page_up(&mut self) {
        self.handler_text = "ctrl + u -> half page up".to_string();
        self.tree_state.select_relative(|c| c.map_or(0, |c| c.saturating_sub(10)));
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

    fn open_all(&mut self) {
        self.handler_text = "shift + E -> expand all".to_string();
        self.tree_state.flatten(&self.tree_items).iter().for_each(|node| {
            self.tree_state.open(node.identifier.clone());
        });
    }

    fn close_all(&mut self) {
        self.handler_text = "shift + C -> collapse all".to_string();
        self.tree_state.close_all();
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

fn build_tree_items(tags: &GroupedTags) -> Vec<TreeItem<'static, String>> {
    use dicom_core::DataDictionary;
    let dict = dicom_dictionary_std::StandardDataDictionary::default();

    tags.iter()
        .map(|(group, elements)| {
            let group_text = format!("Group {:#06x}", group);
            let group_id = format!("group_{:04x}", group);

            let children: Vec<TreeItem<String>> = elements
                .iter()
                .enumerate()
                .map(|(idx, tag_elem)| {
                    let tag = tag_elem.header().tag;
                    let tag_info_str = if let Some(tag_info) = dict.by_tag(tag) {
                        format!("{:#06x} '{}' ({})", tag.element(), tag_info.alias, tag_elem.vr())
                    } else {
                        format!("{:#06x} <unknown> ({})", tag.element(), tag_elem.vr())
                    };

                    let value_str = match tag_elem.value() {
                        dicom_core::DicomValue::Primitive(primitive_value) => {
                            if tag_elem.vr() != dicom_core::VR::OB && tag_elem.vr() != dicom_core::VR::OW {
                                let value_str = primitive_value.to_string();
                                if value_str.len() > 80 {
                                    format!(": {}...", &value_str[..77])
                                } else {
                                    format!(": {}", value_str)
                                }
                            } else {
                                String::new()
                            }
                        }
                        dicom_core::DicomValue::Sequence(seq) => format!(": sequence with {} items", seq.items().len()),
                        dicom_core::DicomValue::PixelSequence(_) => ": pixel sequence".to_string(),
                    };

                    let full_text = format!("{}{}", tag_info_str, value_str);
                    let child_id = format!("{}_elem_{}", group_id, idx);
                    TreeItem::new_leaf(child_id, full_text)
                })
                .collect();

            TreeItem::new(group_id, group_text, children).expect("all child identifiers are unique")
        })
        .collect()
}

impl<'a> Widget for &mut App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![" DICOM Tagger - ".bold(), self.input_file.into(), " ".into()]);
        // let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);

        let [list_area, state_area, input_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(2), Constraint::Length(2)]).areas(area);

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
        Paragraph::new("").block(input_block).render(input_area, buf);

        // Render help overlay if shown
        if self.show_help {
            self.render_help_overlay(area, buf);
        }
    }
}

pub const fn help_text() -> &'static str {
    r#"Navigation:
  k/↑/ctrl+p     - Move up
  j/↓/ctrl+n     - Move down
  ctrl+u         - Move half page up
  ctrl+d         - Move half page down
  g              - Move to first element
  G              - Move to last element
  Enter/Space    - Toggle expand/collapse
  ?              - Show help
  q/Esc          - Quit
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_tree_structure() {
        let mut tags = BTreeMap::new();
        let elements = vec![];
        tags.insert(0x0008, elements.clone());
        tags.insert(0x0010, elements);

        let tree_items = build_tree_items(&tags);

        assert_eq!(tree_items.len(), 2, "Should have 2 groups");

        assert_eq!(tree_items[0].identifier(), "group_0008");
        assert_eq!(tree_items[1].identifier(), "group_0010");
    }
}
