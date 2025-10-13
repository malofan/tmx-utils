use anyhow::{Context, Result};
use quick_xml::events::{BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::Writer;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::whitespace::is_xml_whitespace;
use crate::attribute::get_attribute_value;

fn write_t5_n_tag(writer: &mut Writer<BufWriter<File>>, e: &quick_xml::events::BytesStart, unprotect: bool) -> Result<()> {
    if ! unprotect {
        writer.write_event(Event::Empty(e.clone()))?;
    } else if let Some(n_value) = get_attribute_value(&e, b"n") {
        let n_str = String::from_utf8_lossy(&n_value);
        writer.write_event(Event::Text(BytesText::from_escaped(n_str.as_ref())))?;
    } else {
        writer.write_event(Event::Empty(e.clone()))?;
    }

    Ok(())
}

/// Concatenate TMX files by merging all <tu> nodes from subsequent files into the first one's <body>.
pub fn concat(files: &[String], output: &String, unprotect: bool) -> Result<()> {
    if files.is_empty() {
        return Err(anyhow::anyhow!("No input files provided"));
    }

    // Open first file and parse header, <body>, and <tu> nodes
    let first_file = File::open(&files[0]).context("Failed to open first input file")?;
    let mut reader = Reader::from_reader(BufReader::new(first_file));
    reader.config_mut().trim_text(false);

    let output_file = File::create(output).context("Failed to create output file")?;
    let mut writer = Writer::new(BufWriter::new(output_file));

    let mut buf = Vec::new();
    let mut in_body = false;

    // Write header and <body> start from first file
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"body" => {
                writer.write_event(Event::Start(e.clone()))?;
                in_body = true;
                break;
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"body" => {
                writer.write_event(Event::Empty(e.clone()))?;
                break;
            }
            Ok(Event::Eof) => return Err(anyhow::anyhow!("Malformed TMX: <body> not found")),
            Ok(ev) => writer.write_event(ev.clone())?,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
        }
        buf.clear();
    }
    buf.clear();

    // Write all <tu> nodes from first file
    if in_body {
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"tu" => {
                    writer.write_event(Event::Start(e.clone()))?;
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"tu" => {
                    writer.write_event(Event::Empty(e.clone()))?;
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"tu" => {
                    writer.write_event(Event::End(e.clone()))?;
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"body" => {
                    // Don't write </body> yet
                    break;
                }
                Ok(Event::Eof) => return Err(anyhow::anyhow!("Malformed TMX: <body> not closed")),
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"t5:n" => {
                    write_t5_n_tag(&mut writer, e, unprotect)?
                },
                Ok(ev) => writer.write_event(ev.clone())?,
                Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            }
            buf.clear();
        }
    }
    buf.clear();

    // For each subsequent file, extract <tu> nodes and write them into output
    for file in &files[1..] {
        let input_file = File::open(file).context("Failed to open input file")?;
        let mut reader = Reader::from_reader(BufReader::new(input_file));
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut just_skipped_node = false;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"tu" => {
                    writer.write_event(Event::Start(e.clone()))?;
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"tu" => {
                    writer.write_event(Event::Empty(e.clone()))?;
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"tu" => {
                    writer.write_event(Event::End(e.clone()))?;
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"t5:n" => {
                    write_t5_n_tag(&mut writer, e, unprotect)?
                },
                Ok(Event::Decl(_)) => { just_skipped_node = true; }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"tmx" => { just_skipped_node = true; }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"tmx" => { just_skipped_node = true; }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"tmx" => { just_skipped_node = true; }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"header" => { just_skipped_node = true; }
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"body" => { just_skipped_node = true; }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"body" => {
                    // Don't write </body> yet
                    break;
                }
                Ok(Event::Text(ref t)) => {
                    if just_skipped_node && is_xml_whitespace(t.as_ref()) {
                        // skip writing this whitespace
                        just_skipped_node = false;

                        continue;
                    }
                    writer.write_event(Event::Text(t.clone()))?;
                }
                Ok(Event::Eof) => return Err(anyhow::anyhow!("Malformed TMX: <body> not closed")),
                Ok(ev) => {
                    writer.write_event(ev.clone())?
                },
                Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            }
            buf.clear();
        }
    }

    // Write </body> and the rest of the first file (footer)
    writer.write_event(Event::Text(BytesText::from_escaped("</body>\r</tmx>")))?;

    Ok(())
}