use dicom_core::{DataElement, PrimitiveValue, VR, header::Header};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};
use tui_textarea::{Input, TextArea};

use crate::dicom::{self, TagElement};

/// How the editor behaves for a given VR.
#[derive(Debug, Clone, PartialEq)]
enum EditMode {
    /// Multiline text (LT, ST, UT)
    Multiline,
    /// Single-line text with no further constraints
    SingleLine,
    /// Unsigned integer with a maximum value (UL → u32::MAX, US → u16::MAX, UV → u64::MAX)
    UnsignedInt { max: u64 },
    /// Signed integer with min/max (SL, SS, SV)
    SignedInt { min: i64, max: i64 },
    /// Floating-point number (FL → f32, FD → f64, DS)
    Float { is_f32: bool },
}

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
    edit_mode: EditMode,
    error_msg: Option<String>,
}

impl TagEdit {
    pub fn new(element: &TagElement) -> Self {
        let element = element.clone();
        let edit_mode = edit_mode_for_vr(element.vr());
        let lines = initial_text(&element);

        let mut text_area = TextArea::new(lines);
        text_area.move_cursor(tui_textarea::CursorMove::End);
        text_area.set_cursor_line_style(Style::default());

        Self {
            element,
            text_area,
            edit_mode,
            error_msg: None,
        }
    }

    /// Returns the editing [`State`] after processing the key:
    /// - [`State::Canceled`] on Esc – discard changes
    /// - [`State::Updated`]  on (Alt+)Enter – carries the rebuilt [`TagElement`]
    /// - [`State::Editing`]  otherwise – key was forwarded to the text area
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> State {
        match key_event.code {
            KeyCode::Esc => State::Canceled,
            KeyCode::Enter => {
                // Alt+Enter in multiline mode confirms; plain Enter inserts a newline.
                // Single-line/numeric: plain Enter always confirms.
                match &self.edit_mode {
                    EditMode::Multiline => {
                        if key_event.modifiers.contains(KeyModifiers::ALT) {
                            self.try_commit()
                        } else {
                            self.text_area.input(Input::from(key_event));
                            State::Editing
                        }
                    }
                    _ => self.try_commit(),
                }
            }
            KeyCode::Char(c) => {
                // For numeric VRs, filter characters before forwarding to textarea
                let current_line = self.text_area.lines()[self.text_area.cursor().0].clone();
                let cursor_col = self.text_area.cursor().1;

                let accepted = match &self.edit_mode {
                    EditMode::UnsignedInt { .. } => is_unsigned_char(c),
                    EditMode::SignedInt { .. } => is_signed_char(c, &current_line, cursor_col),
                    EditMode::Float { .. } => is_float_char(c, &current_line),
                    _ => true,
                };

                if accepted {
                    self.text_area.input(Input::from(key_event));
                    self.error_msg = None;
                }

                State::Editing
            }
            _ => {
                self.text_area.input(Input::from(key_event));
                State::Editing
            }
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let is_multiline = self.edit_mode == EditMode::Multiline;
        // For multiline we allow up to 8 lines of text input
        let text_lines = if is_multiline {
            (self.text_area.lines().len() as u16).clamp(3, 8)
        } else {
            1u16
        };

        let error_rows = if self.error_msg.is_some() { 1u16 } else { 0 };
        let popup_height = 2 + 4 + error_rows + text_lines;
        let popup_width = (area.width as f32 * 0.65) as u16;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + popup_x,
            y: area.y + popup_y,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        let hint = format!(" {}Enter: confirm  Esc: cancel ", if is_multiline { "Alt+" } else { "" });
        let outer_block = Block::bordered()
            .padding(Padding::horizontal(1))
            .title(Line::from(" Edit Tag ".bold()).centered())
            .title_bottom(Line::from(hint.dark_gray()).right_aligned())
            .border_set(border::ROUNDED);

        let inner_area = outer_block.inner(popup_area);
        outer_block.render(popup_area, buf);

        // Build vertical layout inside the popup
        let mut constraints = vec![
            Constraint::Length(1), // group
            Constraint::Length(1), // element
            Constraint::Length(1), // name
            Constraint::Length(1), // vr
        ];
        if self.error_msg.is_some() {
            constraints.push(Constraint::Length(1)); // error
        }
        constraints.push(Constraint::Length(text_lines)); // input

        let rows = Layout::vertical(constraints).split(inner_area);

        let tag = self.element.header().tag();
        Paragraph::new(Line::from(vec!["Group:   ".bold(), format!("0x{:04X}", tag.group()).into()])).render(rows[0], buf);
        Paragraph::new(Line::from(vec!["Element: ".bold(), format!("0x{:04X}", tag.element()).into()])).render(rows[1], buf);
        Paragraph::new(Line::from(vec!["Name:    ".bold(), dicom::get_tag_name(&self.element).into()])).render(rows[2], buf);

        let vr_hint = self.vr_hint();
        Paragraph::new(Line::from(vec![
            "VR:      ".bold(),
            self.element.vr().to_string().into(),
            "  ".into(),
            vr_hint.dark_gray(),
        ]))
        .render(rows[3], buf);

        // Optional error msg row
        let input_row_idx = if let Some(err) = &self.error_msg {
            Paragraph::new(Line::from(vec![err.clone().into()]))
                .style(Style::default().fg(Color::Red))
                .render(rows[4], buf);
            5
        } else {
            4
        };

        // Value label + textarea
        let [label_col, input_col] = Layout::horizontal([Constraint::Length(9), Constraint::Min(0)]).areas(rows[input_row_idx]);
        let label_text = if is_multiline { "Value:\n      \n      " } else { "Value:   " };
        Paragraph::new(label_text.bold()).render(label_col, buf);
        self.text_area.render(input_col, buf);
    }

    fn try_commit(&mut self) -> State {
        let lines = self.text_area.lines();

        // Validate numeric VRs before accepting
        match &self.edit_mode {
            EditMode::UnsignedInt { max } => {
                let text = lines[0].trim().to_string();
                if let Err(msg) = validate_unsigned(&text, *max) {
                    self.error_msg = Some(msg);
                    return State::Editing;
                }
            }
            EditMode::SignedInt { min, max } => {
                let text = lines[0].trim().to_string();
                if let Err(msg) = validate_signed(&text, *min, *max) {
                    self.error_msg = Some(msg);
                    return State::Editing;
                }
            }
            EditMode::Float { is_f32 } => {
                let text = lines[0].trim().to_string();
                if let Err(msg) = validate_float(&text, *is_f32) {
                    self.error_msg = Some(msg);
                    return State::Editing;
                }
            }
            _ => {}
        }

        self.error_msg = None;

        // Reconstruct the new value from the textarea lines.
        // Multiline VRs join with the DICOM text continuation character "\r\n".
        let new_value: String = if self.edit_mode == EditMode::Multiline {
            lines.join("\r\n")
        } else {
            lines[0].clone()
        };

        let updated = DataElement::new(
            self.element.header().tag(),
            self.element.vr(),
            PrimitiveValue::from(new_value.as_str()),
        );
        State::Updated(updated)
    }

    /// Short human-readable hint describing the VR constraints.
    fn vr_hint(&self) -> String {
        match &self.edit_mode {
            EditMode::Multiline => "(multiline text — Alt+Enter to confirm)".to_string(),
            EditMode::SingleLine => "(text)".to_string(),
            EditMode::UnsignedInt { max } => format!("(unsigned integer, 0 – {})", max),
            EditMode::SignedInt { min, max } => format!("(signed integer, {} – {})", min, max),
            EditMode::Float { is_f32: true } => "(32-bit float)".to_string(),
            EditMode::Float { is_f32: false } => "(64-bit float / decimal string)".to_string(),
        }
    }
}

/// Returns `true` for VRs whose raw bytes are not meaningful to a human editor.
pub fn is_binary_vr(vr: VR) -> bool {
    matches!(vr, VR::OB | VR::OD | VR::OF | VR::OL | VR::OV | VR::OW | VR::UN)
}

fn edit_mode_for_vr(vr: VR) -> EditMode {
    match vr {
        // Multiline text
        VR::LT | VR::ST | VR::UT => EditMode::Multiline,

        // Unsigned integers
        VR::UL => EditMode::UnsignedInt { max: u32::MAX as u64 },
        VR::US => EditMode::UnsignedInt { max: u16::MAX as u64 },
        VR::UV => EditMode::UnsignedInt { max: u64::MAX },

        // Signed integers
        VR::SL => EditMode::SignedInt {
            min: i32::MIN as i64,
            max: i32::MAX as i64,
        },
        VR::SS => EditMode::SignedInt {
            min: i16::MIN as i64,
            max: i16::MAX as i64,
        },
        VR::SV => EditMode::SignedInt {
            min: i64::MIN,
            max: i64::MAX,
        },

        // Floating point
        VR::FL => EditMode::Float { is_f32: true },
        VR::FD => EditMode::Float { is_f32: false },
        VR::DS => EditMode::Float { is_f32: false },

        // Everything else is treated as single-line text
        _ => EditMode::SingleLine,
    }
}

/// Initial text to show in the editor for a given element.
fn initial_text(element: &TagElement) -> Vec<String> {
    let raw = dicom::get_value_string(element);
    // Normalize line endings (\r\n or \r → \n) then split.
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(|s| s.to_string())
        .collect()
}

fn validate_unsigned(text: &str, max: u64) -> Result<(), String> {
    match text.trim().parse::<u64>() {
        Ok(v) if v <= max => Ok(()),
        Ok(_) => Err(format!("Value must be in [0, {}]", max)),
        Err(_) => Err("Enter a non-negative integer".to_string()),
    }
}

fn validate_signed(text: &str, min: i64, max: i64) -> Result<(), String> {
    match text.trim().parse::<i64>() {
        Ok(v) if v >= min && v <= max => Ok(()),
        Ok(_) => Err(format!("Value must be in [{}, {}]", min, max)),
        Err(_) => Err(format!("Enter an integer in [{}, {}]", min, max)),
    }
}

fn validate_float(text: &str, is_f32: bool) -> Result<(), String> {
    let trimmed = text.trim();
    if is_f32 {
        trimmed
            .parse::<f32>()
            .map(|_| ())
            .map_err(|_| "Enter a valid 32-bit floating-point number".to_string())
    } else {
        trimmed
            .parse::<f64>()
            .map(|_| ())
            .map_err(|_| "Enter a valid floating-point number".to_string())
    }
}

/// Is this character admissible while typing an unsigned integer?
fn is_unsigned_char(c: char) -> bool {
    c.is_ascii_digit()
}

/// Is this character admissible while typing a signed integer?
fn is_signed_char(c: char, current_text: &str, cursor_col: usize) -> bool {
    if c == '-' {
        // Only allow '-' at the very beginning and only one of them
        cursor_col == 0 && !current_text.contains('-')
    } else {
        c.is_ascii_digit()
    }
}

/// Is this character admissible while typing a float?
fn is_float_char(c: char, current_text: &str) -> bool {
    match c {
        '0'..='9' => true,
        '-' => !current_text.contains('-'),
        '+' => !current_text.contains('+') && !current_text.contains('-'),
        '.' => !current_text.contains('.'),
        'e' | 'E' => !current_text.to_lowercase().contains('e'),
        _ => false,
    }
}
