use anyhow::{Context, Result};

use crate::concat::concat;

pub fn concat_dir(input_dir: &String, output: &String, unprotect: bool) -> Result<()> {
    let mut input_files: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(input_dir).context("Cannot read input directory")? {
        let entry = entry.context("Cannot read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "tmx" {
                    input_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    if input_files.is_empty() {
        return Err(anyhow::anyhow!("No .tmx files found in the input directory"));
    }

    concat(&input_files, output, unprotect)
}