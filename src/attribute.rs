pub fn get_attribute_value(e: &quick_xml::events::BytesStart, attr_name: &[u8]) -> Option<Vec<u8>> {
    for attr in e.attributes().with_checks(false).filter_map(|a| a.ok()) {
        if attr.key.as_ref() == attr_name {
            return Some(attr.value.into_owned());
        }
    }
    None
}