use std::path::Path;
use std::{fs, io};

use anyhow::Result;
use dicom_object::InMemDicomObject;
use tui_tree_widget::TreeItem;

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;

#[derive(Debug, Clone)]
pub struct DatasetEntry {
    pub filename: String,
    pub dataset: dicom_object::FileDicomObject<InMemDicomObject>,
}

pub fn parse_dicom_files(path: &Path) -> Result<Vec<DatasetEntry>> {
    let mut datasets_with_filename = Vec::new();

    if path.is_dir() {
        let mut dir_entries = fs::read_dir(path)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;
        dir_entries.sort();

        for entry_path in &dir_entries {
            if entry_path.is_dir() {
                continue;
            }

            let dataset = dicom_object::OpenFileOptions::new()
                .read_until(dicom_dictionary_std::tags::PIXEL_DATA)
                .open_file(entry_path)?;
            let filename = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

            datasets_with_filename.push(DatasetEntry { filename, dataset });
        }
    } else {
        let dataset = dicom_object::open_file(path)?;
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

        datasets_with_filename.push(DatasetEntry { filename, dataset });
    }

    Ok(datasets_with_filename)
}

pub fn tree_by_filename(root_dir: &str, datasets_with_filename: &[DatasetEntry]) -> tui_tree_widget::TreeItem<'static, String> {
    let mut root_node = TreeItem::new("root".to_string(), root_dir.to_string(), Vec::new()).expect("valid root");

    for entry in datasets_with_filename {
        let file_id = format!("file_{}", entry.filename.replace(['.', ' '], "_"));
        let mut file_node = TreeItem::new(file_id.clone(), entry.filename.clone(), Vec::new()).expect("valid file");

        let mut current_group_node: Option<TreeItem<'_, String>> = None;
        let mut current_group = 0u16;

        for elem in entry.dataset.iter() {
            let tag = elem.header().tag;

            if current_group != tag.group() {
                if let Some(cgn) = &current_group_node {
                    file_node.add_child(cgn.clone()).expect("valid group"); // set prev
                }
                current_group = tag.group();
                let group_text = format!("{:04x}", current_group);
                let group_id = format!("{}_{:04x}", file_id, current_group);
                current_group_node = Some(TreeItem::new(group_id, group_text, Vec::new()).expect("valid group"));
            }

            let tag_name = get_tag_name(elem);
            let value = get_value_string(elem);
            let element_text = format!(
                "{:04x} {} ({}, {}): {}",
                tag.element(),
                tag_name,
                elem.vr(),
                elem.header().len,
                value
            );
            let elem_id = format!("{}_elem_{:04x}_{:04x}", file_id, tag.group(), tag.element());
            let element_node = TreeItem::new_leaf(elem_id, element_text);
            current_group_node.as_mut().unwrap().add_child(element_node).expect("valid element");
        }

        file_node.add_child(current_group_node.unwrap()).expect("valid group"); // add last group
        root_node.add_child(file_node).expect("valid group");
    }

    root_node
}

fn get_tag_name(elem: &crate::dicom::TagElement) -> String {
    use dicom_core::DataDictionary;
    let dict = dicom_dictionary_std::StandardDataDictionary;
    if let Some(tag_info) = dict.by_tag(elem.header().tag) {
        tag_info.alias.to_string()
    } else {
        "<unknown>".to_string()
    }
}

fn get_value_string(elem: &crate::dicom::TagElement) -> String {
    match elem.value() {
        dicom_core::DicomValue::Primitive(primitive_value) => {
            if elem.vr() != dicom_core::VR::OB && elem.vr() != dicom_core::VR::OW {
                let value_str = primitive_value.to_string();
                if value_str.len() > 80 {
                    format!("{}...", &value_str[..77])
                } else {
                    value_str
                }
            } else {
                "<binary data>".to_string()
            }
        }
        dicom_core::DicomValue::Sequence(seq) => {
            format!("sequence with {} items", seq.items().len())
        }
        dicom_core::DicomValue::PixelSequence(_) => "pixel sequence".to_string(),
    }
}
