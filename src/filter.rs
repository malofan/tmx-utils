use anyhow::{Context, Result};
use quick_xml::events::{BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::Writer;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufReader, BufWriter};
use chrono::{NaiveDateTime, DateTime, Utc};

use crate::attribute::get_attribute_value;

pub struct SkipOptions {
    pub(crate) skip_author: bool,
    pub(crate) skip_document: bool,
    pub(crate) skip_context: bool,
    pub(crate) keep_diff_targets: bool,
}

#[derive(Clone)]
enum TmxNode<'a> {
    Start(quick_xml::events::BytesStart<'a>),
    End(quick_xml::events::BytesEnd<'a>),
    Event(quick_xml::events::Event<'a>),
    Text(quick_xml::events::BytesText<'a>),
}

struct Tu<'a> {
    timestamp: i64,
    tu_content: Vec<TmxNode<'a>>,
}

pub fn filter(
    input: &String,
    output: &String,
    skip_options: SkipOptions
) -> Result<()> {
    // Usage: tmx_trimmer <input.tmx> <output.tmx> <N>
    let infile = File::open(input).context("Cannot open input file")?;
    let mut reader = Reader::from_reader(BufReader::new(infile));
    // Keep whitespace as-is globally; we’ll selectively drop only the ws after skipped <tu>.
    reader.config_mut().trim_text(false);

    let outfile = File::create(output).context("Cannot create output file")?;
    let mut writer = Writer::new(BufWriter::new(outfile));

    let mut inside_tu = false;
    let mut inside_doc_prop= false;
    let mut inside_context_prop= false;
    let mut inside_source_tuv = false;
    let mut inside_source_seg = false;
    let mut inside_target_seg = false;

    // tu map. key is hash of fields and value is TU node
    let mut tu_map: std::collections::HashMap<u64, Tu> = std::collections::HashMap::new();

    let mut author_value: Option<Vec<u8>> = None;
    let mut document_value: Option<Vec<u8>> = None;
    let mut context_value: Option<Vec<u8>> = None;
    let mut timestamp_value: Option<Vec<u8>> = None;
    let mut source_lang: Option<Vec<u8>> = None;

    let mut buf = Vec::new();
    let mut tu_content: Vec<TmxNode<'_>> = Vec::new();
    let mut source_content: Vec<TmxNode<'_>> = Vec::new();
    let mut target_content: Vec<TmxNode<'_>> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(ref e)) => writer.write_event(Event::Decl(e.clone()))?,
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"tmx" => {
                writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;
                writer.write_event(Event::Start(e.clone()))?
            },
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"header" => {
                writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;
                writer.write_event(Event::Empty(e.clone()))?
            },
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"body" => {
                writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;
                writer.write_event(Event::Start(e.clone()))?
            },

            Ok(Event::Eof) => break,

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"tu" => {
                inside_tu = true;

                tu_content.clear();
                source_content.clear();
                target_content.clear();

                document_value = None;
                context_value = None;

                tu_content.push(TmxNode::Start(e.clone().into_owned()));

                author_value = get_attribute_value(&e, b"creationid");
                timestamp_value = get_attribute_value(&e, b"creationdate");
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"prop" => {
                if !inside_tu {
                    continue;
                }

                tu_content.push(TmxNode::Start(e.clone().into_owned()));

                if let Some(type_attr) = get_attribute_value(&e, b"type") {
                    if type_attr.as_ref() as &[u8] == b"tmgr:docname" {
                        inside_doc_prop = true;
                        inside_context_prop = false;
                        continue;
                    } else if type_attr.as_ref() as &[u8] == b"tmgr:context" {
                        inside_doc_prop = false;
                        inside_context_prop = true;
                        continue;
                    }
                }
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"tuv" => {
                if !inside_tu {
                    continue;
                }

                tu_content.push(TmxNode::Start(e.clone().into_owned()));

                if let Some(lang) = get_attribute_value(&e, b"xml:lang") {
                    if source_lang.is_none() {
                        source_lang = Some(lang.clone());
                    }

                    inside_source_tuv = source_lang.as_ref().map_or(false, |sl| &lang == sl);
                }
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"seg" => {
                if !inside_tu {
                    continue;
                }

                tu_content.push(TmxNode::Start(e.clone().into_owned()));

                inside_source_seg = inside_source_tuv;
                inside_target_seg = !inside_source_tuv;
            }

            Ok(Event::End(ref e)) if e.name().as_ref() == b"seg" => {
                tu_content.push(TmxNode::End(e.clone().into_owned()));

                inside_source_seg = false;
                inside_target_seg = false;
            }

            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"tu" => {}

            Ok(Event::End(ref e)) if e.name().as_ref() == b"tu" => {
                inside_tu = false;

                let mut hasher = std::collections::hash_map::DefaultHasher::new();

                // hash source content and other fields unless skipped

                for node in &source_content {
                    match node {
                        TmxNode::Start(e) => std::hash::Hash::hash_slice(e.name().as_ref(), &mut hasher),
                        TmxNode::End(e) => std::hash::Hash::hash_slice(e.name().as_ref(), &mut hasher),
                        TmxNode::Event(ev) => std::hash::Hash::hash_slice(ev.as_ref(), &mut hasher),
                        TmxNode::Text(t) => std::hash::Hash::hash_slice(t.as_ref(), &mut hasher),
                    }
                }

                if !skip_options.skip_author {
                    if let Some(author) = &author_value {
                        let upper = author.iter().map(|b| b.to_ascii_uppercase()).collect::<Vec<_>>();
                        std::hash::Hash::hash_slice(&upper, &mut hasher);
                    }
                }
                if !skip_options.skip_document {
                    if let Some(document) = &document_value {
                        std::hash::Hash::hash_slice(document, &mut hasher);
                    }
                }
                if !skip_options.skip_context {
                    // if no context, hash "-"
                    match &context_value {
                        Some(context) => {
                            if context.is_empty() {
                                std::hash::Hash::hash_slice(b"-", &mut hasher);
                            } else {
                                std::hash::Hash::hash_slice(context, &mut hasher);
                            }
                        },
                        None => std::hash::Hash::hash_slice(b"-", &mut hasher),
                    }
                }

                if skip_options.keep_diff_targets {
                    for node in &target_content {
                        match node {
                            TmxNode::Start(e) => std::hash::Hash::hash_slice(e.name().as_ref(), &mut hasher),
                            TmxNode::End(e) => std::hash::Hash::hash_slice(e.name().as_ref(), &mut hasher),
                            TmxNode::Event(ev) => std::hash::Hash::hash_slice(ev.as_ref(), &mut hasher),
                            TmxNode::Text(t) => std::hash::Hash::hash_slice(t.as_ref(), &mut hasher),
                        }
                    }
                }

                let hash = hasher.finish();

                tu_content.push(TmxNode::End(e.clone().into_owned()));

                let mut tu_timestamp: i64 = 0;

                // parse timestamp from string like 20160323T152428Z
                if let Some(ts) = &timestamp_value {
                    let ts_str = std::str::from_utf8(ts)
                        .context("Failed to parse timestamp as UTF-8")?;

                    let naive = NaiveDateTime::parse_from_str(&ts_str[..15], "%Y%m%dT%H%M%S")?;
                    let date_time = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);

                    tu_timestamp = date_time.timestamp();
                }

                if !tu_map.contains_key(&hash) {
                    // write TU node
                    let tu = Tu {
                        timestamp: tu_timestamp,
                        tu_content: tu_content.clone(),
                    };
                    tu_map.insert(hash, tu);

                    continue;
                }

                // if new TU has more recent timestamp, replace existing TU
                if let Some(tu) = tu_map.get_mut(&hash) {
                    if tu_timestamp > tu.timestamp {
                        tu.timestamp = tu_timestamp;
                        tu.tu_content = tu_content.clone();
                    }
                }   
            }

            Ok(Event::Text(ref t)) => {
                if !inside_tu {
                    continue;
                }

                tu_content.push(TmxNode::Text(t.clone().into_owned()));

                if inside_source_seg {
                    source_content.push(TmxNode::Text(t.clone().into_owned()));
                }

                if inside_target_seg {
                    target_content.push(TmxNode::Text(t.clone().into_owned()));
                }

                if inside_doc_prop {
                    document_value = Some(t.as_ref().to_vec());
                    inside_doc_prop = false;
                    continue;
                } else if inside_context_prop {
                    context_value = Some(t.as_ref().to_vec());
                    inside_context_prop = false;
                    continue;
                }
            }

            Ok(ev) => {
                if inside_tu {
                    if inside_source_seg {
                        source_content.push(TmxNode::Event(ev.clone().into_owned()));
                    }

                    if inside_target_seg {
                        target_content.push(TmxNode::Event(ev.clone().into_owned()));
                    }

                    // accumulate TU content
                    tu_content.push(TmxNode::Event(ev.into_owned()));
                }

                continue;
            }

            Err(e) => return Err(anyhow::anyhow!(
                "XML parse error at {:?}: {}",
                reader.error_position(),
                e
            )),
        }
        buf.clear();
    }

    // write all TU nodes from map sorted by timestamp
    let mut tu_list: Vec<&Tu> = tu_map.values().collect();
    // sort by timestamp ascending
    tu_list.sort_by_key(|tu| tu.timestamp);

    for tu in tu_list {
        for node in &tu.tu_content {
            match node {
                TmxNode::Start(e) => {
                    // if start of tu tag -> write newline before
                    if e.name().as_ref() == b"tu" {
                        writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;
                    }
                    writer.write_event(Event::Start(e.clone()))?
                },
                TmxNode::End(e) => writer.write_event(Event::End(e.clone()))?,
                TmxNode::Event(ev) => writer.write_event(ev.clone())?,
                TmxNode::Text(t) => writer.write_event(Event::Text(t.clone()))?,
            }
        }
    }

    writer.write_event(Event::Text(BytesText::from_escaped("\n</body>\n</tmx>")))?;

    Ok(())
}

// tests
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq};

    #[test]
    fn test_filter_no_skip_keep() {
        let skip_options = SkipOptions {
            skip_author: false,
            skip_document: false,
            skip_context: false,
            keep_diff_targets: true,
        };
        let result = filter(&"test-data/filter/test.tmx".to_string(), &"test_no_skip_keep.tmx".to_string(), skip_options);
        assert!(result.is_ok());

        let expected = std::fs::read_to_string("test-data/filter/test_no_skip_keep.tmx").unwrap();
        let output = std::fs::read_to_string("test_no_skip_keep.tmx").unwrap();

        // remove output file after test
        std::fs::remove_file("test_no_skip_keep.tmx").unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn test_filter_no_skip_not_keep() {
        let skip_options = SkipOptions {
            skip_author: false,
            skip_document: false,
            skip_context: false,
            keep_diff_targets: false,
        };
        let result = filter(&"test-data/filter/test.tmx".to_string(), &"output_no_skip_no_keep.tmx".to_string(), skip_options);
        assert!(result.is_ok());

        let expected = std::fs::read_to_string("test-data/filter/test_no_skip_no_keep.tmx").unwrap();
        let output = std::fs::read_to_string("output_no_skip_no_keep.tmx").unwrap();

        assert_eq!(expected, output);

        // remove output file after test
        std::fs::remove_file("output_no_skip_no_keep.tmx").unwrap();
    }

    #[test]
    fn test_filter_skip_author() {
        let skip_options = SkipOptions {
            skip_author: true,
            skip_document: false,
            skip_context: false,
            keep_diff_targets: false,
        };
        let result = filter(&"test.tmx".to_string(), &"output_skip_author.tmx".to_string(), skip_options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_skip_document() {
        let skip_options = SkipOptions {
            skip_author: false,
            skip_document: true,
            skip_context: false,
            keep_diff_targets: false,
        };
        let result = filter(&"test.tmx".to_string(), &"output_skip_document.tmx".to_string(), skip_options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_skip_context() {
        let skip_options = SkipOptions {
            skip_author: false,
            skip_document: false,
            skip_context: true,
            keep_diff_targets: false,
        };
        let result = filter(&"test.tmx".to_string(), &"output_skip_context.tmx".to_string(), skip_options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_keep_diff_targets() {
        let skip_options = SkipOptions {
            skip_author: false,
            skip_document: false,
            skip_context: false,
            keep_diff_targets: true,
        };
        let result = filter(&"test.tmx".to_string(), &"output_keep_diff_targets.tmx".to_string(), skip_options);
        assert!(result.is_ok());
    }
}