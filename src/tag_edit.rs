use dicom_core::{DataElement, Tag};
use dicom_object::InMemDicomObject;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};
use tui_textarea::TextArea;

use crate::dicom;

#[derive(Debug)]
pub struct TagEdit {
    pub tag: Tag,
    pub name: String,
    pub vr: String,
    _element: DataElement<InMemDicomObject>,
    text_area: TextArea<'static>,
}

impl TagEdit {
    pub fn new(tag: Tag, element: &DataElement<InMemDicomObject>) -> Self {
        let element = element.clone();
        let name = dicom::get_tag_name(&element);
        let vr = element.vr().to_string().to_string();
        let current_value = dicom::get_value_string(&element);
        let mut text_area = TextArea::new(vec![current_value]);
        text_area.move_cursor(tui_textarea::CursorMove::End);
        text_area.set_cursor_line_style(Style::default());
        Self {
            tag,
            name,
            vr,
            _element: element,
            text_area,
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

        // Split inner area into: tag info rows + spacer + input field
        let [group_row, element_row, name_row, vr_row, input_row] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner_area);

        Paragraph::new(Line::from(vec!["Group:   ".bold(), format!("0x{:04X}", self.tag.group()).into()])).render(group_row, buf);
        Paragraph::new(Line::from(vec!["Element: ".bold(), format!("0x{:04X}", self.tag.element()).into()])).render(element_row, buf);
        Paragraph::new(Line::from(vec!["Name:    ".bold(), self.name.as_str().into()])).render(name_row, buf);
        Paragraph::new(Line::from(vec!["VR:      ".bold(), self.vr.as_str().into()])).render(vr_row, buf);
        let [value_label_col, value_input_col] = Layout::horizontal([Constraint::Length(9), Constraint::Min(0)]).areas(input_row);
        Paragraph::new(Line::from(vec!["Value:   ".bold()])).render(value_label_col, buf);
        self.text_area.render(value_input_col, buf);
    }
}
