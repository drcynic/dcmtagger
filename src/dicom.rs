use std::collections::BTreeMap;

pub type TagElement = dicom_core::DataElement<dicom_object::InMemDicomObject, Vec<u8>>;
pub type GroupedTags = BTreeMap<u16, Vec<TagElement>>;

pub fn grouped_tags(filename: &str) -> anyhow::Result<GroupedTags> {
    let mut grouped_tags: GroupedTags = BTreeMap::new();
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
