use std::{collections::HashMap, io};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use dicom_core::DataDictionary;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::{self, border},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, ListState, Paragraph, StatefulWidget, Widget},
};

#[derive(Clone, Debug, Parser)]
#[clap(name = "DICOM Tagger", version = format!("v{}", env!("CARGO_PKG_VERSION")))]
#[clap(about = "Copyright (c) 2025 Daniel Szymanski")]
struct Args {
    #[clap(value_parser)]
    input_file: String,
}

fn main() -> anyhow::Result<()> {
    // let args = Args::parse();
    // let input_file = args.input_file;
    let input_file = "testdata/test.dcm".to_string();
    let mut terminal = ratatui::init();
    let app_result = App::new(input_file)?.run(&mut terminal);
    ratatui::restore();
    match app_result {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::format_err!("app error: {e}")),
    }
}

#[derive(Debug, Default)]
pub struct App {
    input_file: String,
    tags: GroupedTags,
    tags_view_state: ListState,
    handler_text: String,
    exit: bool,
    show_help: bool,
    help_scroll_offset: usize,
}

impl App {
    pub fn new(input_file: String) -> anyhow::Result<Self> {
        let tags = grouped_tags(&input_file)?;

        Ok(App {
            input_file,
            tags,
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
        self.tags_view_state.select_next();
    }

    fn move_up(&mut self) {
        self.handler_text = "up".to_string();
        self.tags_view_state.select_previous();
    }

    fn move_half_page_down(&mut self) {
        self.handler_text = "ctrl + d -> half page up".to_string();
        self.move_tag_view_selection(20);
    }

    fn move_half_page_up(&mut self) {
        self.handler_text = "ctrl + u -> half page up".to_string();
        self.move_tag_view_selection(-20);
    }

    fn move_to_first(&mut self) {
        self.handler_text = "g -> move to first".to_string();
        self.tags_view_state.select_first();
    }

    fn move_to_last(&mut self) {
        self.handler_text = "G -> move to last".to_string();
        self.tags_view_state.select_last();
    }

    fn move_tag_view_selection(&mut self, offset: i32) {
        let next = self.tags_view_state.selected().map_or(0, |i| {
            let abs_offset = offset.abs() as usize;
            if offset < 0 {
                i.saturating_sub(abs_offset)
            } else {
                i.saturating_add(abs_offset)
            }
        });
        self.tags_view_state.select(Some(next));
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

        let help_block = Block::bordered().title(" Help".bold()).border_set(border::ROUNDED);

        Paragraph::new(help_text).block(help_block).render(popup_area, buf);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![" DICOM Tagger - ".bold(), self.input_file.clone().into(), " ".into()]);
        // let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);

        let [list_area, state_area, input_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(2), Constraint::Length(2)]).areas(area);

        let bottom_vert_border_set = symbols::border::Set {
            bottom_left: symbols::line::NORMAL.vertical_right,
            bottom_right: symbols::line::NORMAL.vertical_left,
            ..symbols::border::PLAIN
        };

        let list_block = Block::bordered().title(title.centered()).border_set(bottom_vert_border_set);
        let tag_strings = tag_strings(&self.tags);
        let list = ratatui::widgets::List::new(tag_strings)
            .block(list_block)
            .highlight_style(Style::default().bg(Color::DarkGray));
        StatefulWidget::render(list, list_area, buf, &mut self.tags_view_state);

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

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;
pub type GroupedTags = HashMap<u16, Vec<TagElement>>;

pub fn grouped_tags(filename: &str) -> anyhow::Result<GroupedTags> {
    let mut grouped_tags: GroupedTags = HashMap::new();
    let dicom_object = dicom_object::open_file(filename)?;
    for elem in dicom_object {
        let tag_entry = elem.header().tag;
        if let Some(group_elements) = grouped_tags.get_mut(&tag_entry.group()) {
            group_elements.push(elem);
        } else {
            grouped_tags.insert(tag_entry.group(), vec![elem]);
        }
    }

    Ok(grouped_tags)
}

pub fn tag_strings(grouped_tags: &GroupedTags) -> Vec<String> {
    let dict = dicom_dictionary_std::StandardDataDictionary::default();
    grouped_tags
        .iter()
        .flat_map(|(group, elements)| {
            std::iter::once(format!("{:#06x}", group)).chain(elements.iter().map(|tag_elem| {
                let tag = tag_elem.header().tag;
                let tag_info_str = if let Some(tag_info) = dict.by_tag(tag) {
                    format!("    {:#06x} '{}' ({}): ", tag.element(), tag_info.alias, tag_elem.vr())
                } else {
                    format!("    {:#06x} <unknown> ({}): ", tag.element(), tag_elem.vr())
                };

                let value_str = match tag_elem.value() {
                    dicom_core::DicomValue::Primitive(primitive_value) => {
                        if tag_elem.vr() != dicom_core::VR::OB && tag_elem.vr() != dicom_core::VR::OW {
                            primitive_value.to_string()
                        } else {
                            String::new()
                        }
                    }
                    dicom_core::DicomValue::Sequence(seq) => format!("sequence with {} items", seq.items().len()),
                    dicom_core::DicomValue::PixelSequence(_) => "pixel sequence here".to_string(),
                };

                format!("{}{}", tag_info_str, value_str)
            }))
        })
        .collect()
}

pub const fn help_text() -> &'static str {
    r#"DICOM Tagger Help

Navigation:
  k/↑/ctrl+p     - Move up
  j/↓/ctrl+n     - Move down
  ctrl+u         - Move half page up
  ctrl+d         - Move half page down
  g              - Move to first element
  G              - Move to last element
  ?              - Show help
  q/Esc          - Quit
"#
}
