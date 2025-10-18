use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Result;
use dicom_core::{Length, Tag};
use dicom_object::InMemDicomObject;

use crate::tree_widget;

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;

#[derive(Debug, Default)]
pub struct DicomData {
    root_path: PathBuf,
    datasets_with_filename: Vec<DatasetEntry>,
    num_values_and_max_length_by_tag: HashMap<Tag, (usize, Option<u32>)>,
}

#[derive(Debug, Clone)]
pub struct DatasetEntry {
    pub filename: String,
    pub dataset: dicom_object::FileDicomObject<InMemDicomObject>,
}

impl DicomData {
    pub fn new(path: &Path, skip_pixel_data: bool) -> Result<Self> {
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

                datasets_with_filename.push(read_dataset(entry_path, skip_pixel_data)?);
            }
        } else {
            datasets_with_filename.push(read_dataset(path, skip_pixel_data)?);
        }

        let num_values_and_max_length_by_tag = num_distinct_values_and_max_length_by_tag(&datasets_with_filename);

        Ok(Self {
            root_path: PathBuf::from(path),
            datasets_with_filename,
            num_values_and_max_length_by_tag,
        })
    }

    pub fn tree_sorted_by_filename(&self) -> tree_widget::TreeWidget {
        let mut tree_widget = tree_widget::TreeWidget::new(self.root_path.display().to_string());

        if self.root_path.is_dir() {
            for entry in &self.datasets_with_filename {
                let parent_id = tree_widget.add_child(&entry.filename, tree_widget.root_id);
                read_data_into_tree(&mut tree_widget, entry, parent_id);
            }
        } else {
            let parent_id = tree_widget.root_id;
            read_data_into_tree(&mut tree_widget, &self.datasets_with_filename[0], parent_id);
        }

        tree_widget
    }

    pub fn tree_sorted_by_tag(&self, min_diff: usize) -> tree_widget::TreeWidget {
        if self.datasets_with_filename.len() == 1 {
            return self.tree_sorted_by_filename();
        }

        let mut tree_widget = tree_widget::TreeWidget::new(self.root_path.display().to_string());
        let root_id = tree_widget.root_id;

        let mut group_nodes_by_tag_group: BTreeMap<u16, slotmap::DefaultKey> = BTreeMap::new();
        let mut tag_nodes_id_by_tag: BTreeMap<Tag, slotmap::DefaultKey> = BTreeMap::new();

        for entry in &self.datasets_with_filename {
            for elem in entry.dataset.iter() {
                let tag = elem.header().tag;
                let group_node_id = group_nodes_by_tag_group.entry(tag.group()).or_insert_with(|| {
                    let group_tag_text = format!("{:04x}", tag.group());
                    tree_widget.add_child(&group_tag_text, root_id)
                });
                let (num_values, max_length) = self.num_values_and_max_length_by_tag[&tag];
                if num_values > min_diff {
                    let tag_node_id = tag_nodes_id_by_tag.entry(tag).or_insert_with(|| {
                        let tag_name = get_tag_name(elem);
                        let value_lengths_text = if max_length.is_some() {
                            String::new() // will be done per element
                        } else {
                            format!(", {}", elem.header().len)
                        };
                        let tag_text = format!("{:04x} {} ({}{})", tag.element(), tag_name, elem.vr(), value_lengths_text);
                        tree_widget.add_child(&tag_text, *group_node_id)
                    });
                    let value = get_value_string(elem);
                    let element_len = elem.header().len;
                    let element_len = if element_len.0 == Length::UNDEFINED.0 {
                        5
                    } else {
                        element_len.0 as usize
                    };
                    let field_width = if let Some(max_length) = max_length {
                        max_length as usize
                    } else {
                        element_len
                    };
                    let element_text = if tag == dicom_dictionary_std::tags::PIXEL_DATA {
                        format!("[{}] - {}", element_len, &entry.filename)
                    } else {
                        format!("{:<width$}[{}] - {}", value, element_len, &entry.filename, width = field_width)
                    };
                    tree_widget.add_child(&element_text, *tag_node_id);
                }
            }
        }

        tree_widget
    }
}

fn read_dataset(path: &Path, skip_pixel_data: bool) -> anyhow::Result<DatasetEntry> {
    let dataset = if skip_pixel_data {
        dicom_object::OpenFileOptions::new()
            .read_until(dicom_dictionary_std::tags::PIXEL_DATA)
            .open_file(path)?
    } else {
        dicom_object::open_file(path)?
    };
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

    Ok(DatasetEntry { filename, dataset })
}

fn read_data_into_tree(tree_widget: &mut tree_widget::TreeWidget, entry: &DatasetEntry, parent_id: slotmap::DefaultKey) {
    let mut current_group_node_id = parent_id;
    let mut current_group = 0u16;

    for elem in entry.dataset.iter() {
        let tag = elem.header().tag;

        if current_group != tag.group() {
            current_group = tag.group();
            let group_text = format!("{:04x}", current_group);
            current_group_node_id = tree_widget.add_child(&group_text, parent_id);
        }

        let element_text = format!(
            "{:04x} {} ({}, {}): {}",
            tag.element(),
            get_tag_name(elem),
            elem.vr(),
            elem.header().len,
            get_value_string(elem)
        );
        tree_widget.add_child(&element_text, current_group_node_id);
    }
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

pub fn num_distinct_values_and_max_length_by_tag(datasets_with_filename: &[DatasetEntry]) -> HashMap<Tag, (usize, Option<u32>)> {
    let mut values_by_tag: HashMap<Tag, (HashSet<String>, HashSet<u32>)> = HashMap::new();

    for entry in datasets_with_filename {
        for elem in entry.dataset.iter() {
            let tag = elem.header().tag;

            let values_set = values_by_tag.entry(tag).or_default();
            values_set.0.insert(get_value_string(elem));
            values_set.1.insert(elem.header().len.0);
        }
    }

    values_by_tag
        .iter()
        .map(|(&tag, (values, lengths))| {
            (
                tag,
                (
                    values.len(),
                    if lengths.len() > 1 {
                        Some(*lengths.iter().max().unwrap())
                    } else {
                        None
                    },
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Instant;

    #[test]
    fn test_tree_sorted_by_tag_timing() {
        let test_path = Path::new("spine-phantom");

        // Skip test if path doesn't exist
        if !test_path.exists() {
            println!("Test path '{}' does not exist, skipping test", test_path.display());
            return;
        }

        println!("Loading DICOM data from: {}", test_path.display());

        // Measure DicomData creation time
        let load_start = Instant::now();
        let dicom_data = match DicomData::new(test_path, true) {
            Ok(data) => data,
            Err(e) => {
                println!("Failed to load DICOM data: {}", e);
                return;
            }
        };
        let load_duration = load_start.elapsed();
        println!("DicomData::new() execution time: {:?}", load_duration);
        println!("Loaded {} datasets", dicom_data.datasets_with_filename.len());

        // Measure tree_sorted_by_tag execution time
        let tree_start = Instant::now();
        let _tree = dicom_data.tree_sorted_by_tag(0);
        let tree_duration = tree_start.elapsed();
        println!("tree_sorted_by_tag() execution time: {:?}", tree_duration);

        println!("Total execution time: {:?}", load_duration + tree_duration);
    }
}
