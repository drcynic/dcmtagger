use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;
pub type GroupedTags = BTreeMap<u16, Vec<TagElement>>;
pub type FileGroupedTags = BTreeMap<String, GroupedTags>;

#[derive(Debug, Clone)]
pub enum DicomInput {
    File(GroupedTags),
    Directory(String, FileGroupedTags),
}

pub fn process_dicom_input(input_path: &str) -> anyhow::Result<DicomInput> {
    let path = Path::new(input_path);

    if path.is_file() {
        let tags =
            grouped_tags_from_file(input_path).map_err(|e| anyhow::format_err!("Failed to process DICOM file '{}': {}", input_path, e))?;
        Ok(DicomInput::File(tags))
    } else if path.is_dir() {
        let file_tags = grouped_tags_from_directory(input_path)
            .map_err(|e| anyhow::format_err!("Failed to process DICOM directory '{}': {}", input_path, e))?;
        Ok(DicomInput::Directory(input_path.to_string(), file_tags))
    } else {
        Err(anyhow::format_err!("Input path '{}' is neither a file nor a directory", input_path))
    }
}

fn grouped_tags_from_file(filename: &str) -> anyhow::Result<GroupedTags> {
    let mut grouped_tags: GroupedTags = BTreeMap::new();
    let dicom_object =
        dicom_object::open_file(filename).map_err(|e| anyhow::format_err!("Failed to open DICOM file '{}': {}", filename, e))?;

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

fn grouped_tags_from_directory(dir_path: &str) -> anyhow::Result<FileGroupedTags> {
    let mut file_tags: FileGroupedTags = BTreeMap::new();
    let dir = fs::read_dir(dir_path).map_err(|e| anyhow::format_err!("Failed to read directory '{}': {}", dir_path, e))?;

    for entry in dir {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Try to open as DICOM file
                match grouped_tags_from_file(path.to_str().unwrap()) {
                    Ok(tags) => {
                        file_tags.insert(file_name.to_string(), tags);
                    }
                    Err(_) => {
                        // Skip non-DICOM files silently
                        continue;
                    }
                }
            }
        }
    }

    if file_tags.is_empty() {
        return Err(anyhow::format_err!("No valid DICOM files found in directory '{}'", dir_path));
    }

    Ok(file_tags)
}
