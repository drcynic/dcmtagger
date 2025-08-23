use std::collections::BTreeMap;

use dicom_core::DataDictionary;

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

pub fn tag_strings(grouped_tags: &GroupedTags) -> Vec<String> {
    let dict = dicom_dictionary_std::StandardDataDictionary::default();
    grouped_tags
        .iter()
        .flat_map(|(group, elements)| {
            std::iter::once(format!("{:#06x}", group)).chain(elements.iter().map(|tag_elem| {
                let tag = tag_elem.header().tag;
                let tag_info_str = if let Some(tag_info) = dict.by_tag(tag) {
                    format!("    {:#06x} '{}' ({}): ", tag.element(), tag_info.alias, tag_elem.vr())
                } else {
                    format!("    {:#06x} <unknown> ({}): ", tag.element(), tag_elem.vr())
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
