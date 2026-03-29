use dicom_core::Tag;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};

#[derive(Debug, PartialEq)]
pub struct TagEdit {
    pub tag: Tag,
    pub name: String,
    pub vr: String,
    pub current_value: String,
}

impl TagEdit {
    pub fn new(tag: Tag, name: String, vr: String, current_value: String) -> Self {
        Self {
            tag,
            name,
            vr,
            current_value,
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

        Paragraph::new(Line::from(vec!["Group:   ".bold(), format!("{:04X}", self.tag.group()).into()])).render(group_row, buf);
        Paragraph::new(Line::from(vec!["Element: ".bold(), format!("{:04X}", self.tag.element()).into()])).render(element_row, buf);
        Paragraph::new(Line::from(vec!["Name:    ".bold(), self.name.as_str().into()])).render(name_row, buf);
        Paragraph::new(Line::from(vec!["VR:      ".bold(), self.vr.as_str().into()])).render(vr_row, buf);
        Paragraph::new(Line::from(vec!["Value:   ".bold(), self.current_value.as_str().into()])).render(input_row, buf);
    }
}
