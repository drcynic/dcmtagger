use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Result;
use dicom_core::Tag;
use dicom_object::InMemDicomObject;
use tui_tree_widget::TreeItem;

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;

#[derive(Debug, Default)]
pub struct DicomData {
    root_dir: PathBuf,
    datasets_with_filename: Vec<DatasetEntry>,
    num_values_and_length_by_tag: HashMap<Tag, (usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct DatasetEntry {
    pub filename: String,
    pub dataset: dicom_object::FileDicomObject<InMemDicomObject>,
}

impl DicomData {
    pub fn new(path: &Path) -> Result<Self> {
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

        let num_values_and_length_by_tag = num_distinct_values_and_lengths_by_tag(&datasets_with_filename);

        Ok(Self {
            root_dir: PathBuf::from(path),
            datasets_with_filename,
            num_values_and_length_by_tag,
        })
    }

    pub fn tree_sorted_by_filename(&self) -> tui_tree_widget::TreeItem<'static, String> {
        let mut root_node = TreeItem::new("root".to_string(), self.root_dir.display().to_string(), Vec::new()).expect("valid root");

        for entry in &self.datasets_with_filename {
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

    pub fn tree_sorted_by_tag(&self, min_diff: usize) -> tui_tree_widget::TreeItem<'static, String> {
        if self.datasets_with_filename.len() == 1 {
            return self.tree_sorted_by_filename();
        }

        let mut root_node = TreeItem::new("root".to_string(), self.root_dir.display().to_string(), Vec::new()).expect("valid root");
        let mut group_nodes_by_tag_group: BTreeMap<u16, TreeItem<'_, String>> = BTreeMap::new();
        let mut tag_nodes_by_tag: BTreeMap<Tag, TreeItem<'_, String>> = BTreeMap::new();

        for entry in &self.datasets_with_filename {
            let file_id = format!("file_{}", entry.filename.replace(['.', ' '], "_"));
            for elem in entry.dataset.iter() {
                let tag = elem.header().tag;
                group_nodes_by_tag_group.entry(tag.group()).or_insert_with(|| {
                    let group_tag_text = format!("{:04x}/", tag.group());
                    TreeItem::new(group_tag_text.clone(), group_tag_text, Vec::new()).expect("valid node")
                });
                let (num_values, num_lengths) = self.num_values_and_length_by_tag[&tag];
                if num_values > min_diff {
                    let tag_node: &mut TreeItem<'_, std::string::String> = match tag_nodes_by_tag.get_mut(&tag) {
                        Some(node) => node,
                        None => {
                            let tag_name = get_tag_name(elem);
                            let value_lengths_text = if num_lengths == 1 {
                                format!(", {}", elem.header().len)
                            } else {
                                String::new()
                            };
                            let tag_id = format!("{:04x}_{}", tag.group(), tag.element());
                            let tag_text = format!("{:04x} {} ({}{})", tag.element(), tag_name, elem.vr(), value_lengths_text);
                            let tag_node = TreeItem::new(tag_id, tag_text, Vec::new()).expect("valid node");
                            tag_nodes_by_tag.insert(tag, tag_node);
                            tag_nodes_by_tag.get_mut(&tag).unwrap()
                        }
                    };
                    let value = get_value_string(elem);
                    let element_id = format!("{}_{:04x}_{}", &file_id, tag.group(), tag.element(),);
                    let element_text = format!("{} {} - {}", value, elem.header().len, &entry.filename);
                    let element_node = TreeItem::new(element_id, element_text, Vec::new()).expect("valid node");
                    tag_node.add_child(element_node.clone()).expect("valid element node");
                }
            }
        }

        for (tag, tag_node) in &tag_nodes_by_tag {
            group_nodes_by_tag_group
                .get_mut(&tag.group())
                .unwrap()
                .add_child(tag_node.clone())
                .expect("valid tag node");
        }

        for group_node in group_nodes_by_tag_group.values() {
            root_node.add_child(group_node.clone()).expect("valid group node");
        }

        root_node
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

pub fn num_distinct_values_and_lengths_by_tag(datasets_with_filename: &[DatasetEntry]) -> HashMap<Tag, (usize, usize)> {
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
        .map(|(&tag, (values, lengths))| (tag, (values.len(), lengths.len())))
        .collect()
}
