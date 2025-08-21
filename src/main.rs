use std::{collections::HashMap, io};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use dicom_core::DataDictionary;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
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
    handler_text: String,
    exit: bool,
}

impl App {
    pub fn new(input_file: String) -> anyhow::Result<Self> {
        let tags = get_grouped_tags(&input_file)?;

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

    fn draw(&self, frame: &mut Frame) {
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
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn move_down(&mut self) {
        self.handler_text = "down".to_string();
    }

    fn move_up(&mut self) {
        self.handler_text = "up".to_string();
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![" DICOM Tagger - ".bold(), self.input_file.clone().into(), " ".into()]);
        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);
        // let tag_block = Block::default().title("Tags").render(area, buf);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::PLAIN);

        // let handler_text = Text::from(vec![Line::from(vec!["Value: ".into(), self.handler_text.clone().yellow()])]);
        // Paragraph::new(handler_text).centered().block(block.clone()).render(area, buf);

        let tag_strings = get_tag_strings(&self.tags);
        ratatui::widgets::List::new(tag_strings).block(block).render(area, buf);
    }
}

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;
pub type GroupedTags = HashMap<u16, Vec<TagElement>>;

pub fn get_grouped_tags(filename: &str) -> anyhow::Result<GroupedTags> {
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

pub fn get_tag_strings(grouped_tags: &GroupedTags) -> Vec<String> {
    let dict = dicom_dictionary_std::StandardDataDictionary::default();
    grouped_tags
        .iter()
        .flat_map(|(group, elements)| {
            std::iter::once(format!("{:#06x}", group)).chain(elements.iter().map(|tag_elem| {
                let tag = tag_elem.header().tag;
                let tag_info_str = if let Some(tag_info) = dict.by_tag(tag) {
                    format!("\t{:#06x} '{}' ({}): ", tag.element(), tag_info.alias, tag_elem.vr())
                } else {
                    format!("\t{:#06x} <unknown> ({}): ", tag.element(), tag_elem.vr())
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
