use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::Writer;
use std::fs::File;
use std::hash::Hasher;
use std::io::{BufReader, BufWriter};
use chrono::{DateTime};

use crate::attribute::get_attribute_value;

struct SkipOptions {
    skipAuthor: bool,
    skipDocument: bool,
    skipContext: bool,
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

    // tu map. key is hash of fields and value is TU node
    let mut tu_map: std::collections::HashMap<u64, Tu> = std::collections::HashMap::new();

    let mut author_value: Option<Vec<u8>> = None;
    let mut document_value: Option<Vec<u8>> = None;
    let mut context_value: Option<Vec<u8>> = None;
    let mut timestamp_value: Option<Vec<u8>> = None;

    let mut buf = Vec::new();
    let mut tu_content: Vec<TmxNode<'_>> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"tu" => {
                inside_tu = true;

                tu_content.clear();

                tu_content.push(TmxNode::Start(e.clone()));
                
                author_value = get_attribute_value(&e, b"creationid");
                timestamp_value = get_attribute_value(&e, b"creationdate");
            }

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"prop" => {
                if !inside_tu {
                    continue;
                }

                tu_content.push(TmxNode::Start(e.clone()));

                if let Some(document) = get_attribute_value(&e, b"tmgr:docname") {
                    document_value = Some(document);
                }

                if let Some(context) = get_attribute_value(&e, b"tmgr:context") {
                    context_value = Some(context);
                }
            }

            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"tu" => {}

            Ok(Event::End(ref e)) if e.name().as_ref() == b"tu" => {
                inside_tu = false;

                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                if !skip_options.skipAuthor {
                    if let Some(author) = &author_value {
                        std::hash::Hash::hash_slice(author, &mut hasher);
                    }
                }
                if !skip_options.skipDocument {
                    if let Some(document) = &document_value {
                        std::hash::Hash::hash_slice(document, &mut hasher);
                    }
                }
                if !skip_options.skipContext {
                    if let Some(context) = &context_value {
                        std::hash::Hash::hash_slice(context, &mut hasher);
                    }
                }

                let hash = hasher.finish();

                tu_content.push(TmxNode::End(e.clone()));

                let mut tu_timestamp: i64 = 0;

                // parse timestamp from string like 20160323T152428Z
                if let Some(ts) = &timestamp_value {
                    let ts_str = std::str::from_utf8(ts)
                        .context("Failed to parse timestamp as UTF-8")?;
                    let date_time = DateTime::parse_from_str(ts_str, "%Y%m%dT%H%M%SZ")?;

                    tu_timestamp = date_time.timestamp();
                }

                author_value = None;
                document_value = None;
                context_value = None;
                timestamp_value = None;

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

            Ok(ev) => {
                if inside_tu {
                    // accumulate TU content
                    tu_content.push(TmxNode::Event(ev.clone()));
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

    Ok(())
}
