use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::Writer;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use crate::whitespace::is_xml_whitespace;

pub fn trim(input: &String, output: &String, n: usize) -> Result<()> {
    // Usage: tmx_trimmer <input.tmx> <output.tmx> <N>
    let infile = File::open(input).context("Cannot open input file")?;
    let mut reader = Reader::from_reader(BufReader::new(infile));
    // Keep whitespace as-is globally; we’ll selectively drop only the ws after skipped <tu>.
    reader.config_mut().trim_text(false);

    let outfile = File::create(output).context("Cannot create output file")?;
    let mut writer = Writer::new(BufWriter::new(outfile));

    let mut buf = Vec::new();
    let mut skip_count = 0usize;
    let mut inside_tu = false;
    let mut just_skipped_tu = false;


    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,

            Ok(Event::Start(ref e)) if e.name().as_ref() == b"tu" => {
                inside_tu = true;
                skip_count += 1;
                if skip_count > n {
                    writer.write_event(Event::Start(e.clone()))?;
                    just_skipped_tu = false;
                } else {
                    just_skipped_tu = true;
                }
            }

            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"tu" => {
                skip_count += 1;
                if skip_count > n {
                    writer.write_event(Event::Empty(e.clone()))?;
                    just_skipped_tu = false;
                } else {
                    just_skipped_tu = true;
                }
            }

            Ok(Event::End(ref e)) if e.name().as_ref() == b"tu" => {
                if skip_count > n {
                    writer.write_event(Event::End(e.clone()))?;
                    just_skipped_tu = false;
                } else {
                    just_skipped_tu = true;
                }
                inside_tu = false;
            }

            Ok(Event::Text(ref t)) => {
                // If we just skipped a <tu>, and this is whitespace, do not write it
                if just_skipped_tu && is_xml_whitespace(t.as_ref()) {
                    // skip writing this whitespace
                    just_skipped_tu = false;
                } else if !(inside_tu && skip_count <= n) {
                    writer.write_event(Event::Text(t.clone()))?;
                    just_skipped_tu = false;
                }
            }

            Ok(ev) => {
                if !(inside_tu && skip_count <= n) {
                    writer.write_event(ev.clone())?;
                    just_skipped_tu = false;
                }
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
