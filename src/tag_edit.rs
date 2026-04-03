use dicom_core::{DataElement, header::Header};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};
use tui_textarea::{Input, TextArea};

use crate::dicom::{self, TagElement};

#[derive(Debug)]
pub enum State {
    Editing,
    Canceled,
    Updated(TagElement),
}

#[derive(Debug)]
pub struct TagEdit {
    element: TagElement,
    text_area: TextArea<'static>,
}

impl TagEdit {
    pub fn new(element: &TagElement) -> Self {
        let element = element.clone();
        let mut text_area = TextArea::new(vec![dicom::get_value_string(&element)]);
        text_area.move_cursor(tui_textarea::CursorMove::End);
        text_area.set_cursor_line_style(Style::default());
        Self { element, text_area }
    }

    /// Returns the editing [`State`] after processing the key:
    /// - [`State::Canceled`] on Esc  – discard changes
    /// - [`State::Updated`]  on Enter – carries the rebuilt [`TagElement`]
    /// - [`State::Editing`]  otherwise – key was forwarded to the text area
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> State {
        match key_event.code {
            KeyCode::Esc => State::Canceled,
            KeyCode::Enter => {
                let new_value = self.text_area.lines()[0].clone();
                let updated = DataElement::new(self.element.header().tag(), self.element.vr(), new_value);
                State::Updated(updated)
            }
            _ => {
                self.text_area.input(Input::from(key_event));
                State::Editing
            }
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let popup_width = (area.width as f32 * 0.6) as u16;
        let popup_height = 7u16;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + popup_x,
            y: area.y + popup_y,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        let outer_block = Block::bordered()
            .padding(Padding::horizontal(1))
            .title(Line::from(" Edit Tag ".bold()).centered())
            .border_set(border::ROUNDED);

        let inner_area = outer_block.inner(popup_area);
        outer_block.render(popup_area, buf);

        let [group_row, element_row, name_row, vr_row, input_row] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner_area);

        let tag = self.element.header().tag();
        Paragraph::new(Line::from(vec!["Group:   ".bold(), format!("0x{:04X}", tag.group()).into()])).render(group_row, buf);
        Paragraph::new(Line::from(vec!["Element: ".bold(), format!("0x{:04X}", tag.element()).into()])).render(element_row, buf);
        Paragraph::new(Line::from(vec!["Name:    ".bold(), dicom::get_tag_name(&self.element).into()])).render(name_row, buf);
        Paragraph::new(Line::from(vec!["VR:      ".bold(), self.element.vr().to_string().into()])).render(vr_row, buf);

        let [value_label_col, value_input_col] = Layout::horizontal([Constraint::Length(9), Constraint::Min(0)]).areas(input_row);
        Paragraph::new(Line::from(vec!["Value:   ".bold()])).render(value_label_col, buf);
        self.text_area.render(value_input_col, buf);
    }
}
