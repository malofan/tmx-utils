#[inline]
pub fn is_xml_whitespace(bytes: &[u8]) -> bool {
    // XML whitespace: SP, TAB, CR, LF
    bytes.iter().all(|b| matches!(*b, b' ' | b'\t' | b'\r' | b'\n'))
}