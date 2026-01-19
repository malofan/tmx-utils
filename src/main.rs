use anyhow::{Context, Result};
use std::env;

mod whitespace;

mod attribute;

mod trim;

use self::trim::trim;

mod concat;
use self::concat::concat;

mod filter;

mod concat_dir;
use crate::concat_dir::concat_dir;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // filter out args before the command
    

    let command = &args[1];

    if "trim" == command {
        if args.len() != 5 {
            eprintln!("Usage: {} {} <input.tmx> <output.tmx> <N>", args[0], command);
            std::process::exit(1);
        }

        return trim(&args[2], &args[3], args[4].parse().context("N must be an integer")?);
    }

    if "concat" == command {
        if args.len() < 5 {
            eprintln!("Usage: {} {} <output.tmx> <unprotect: true|false> <input1.tmx> [<input2.tmx> ...]", args[0], command);
            std::process::exit(1);
        }

        let output = &args[2];
        let unprotect = match args[3].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!("unprotect must be 'true' or 'false'. Got '{}'", args[3]);
                std::process::exit(1);
            }
        };
        let input_files = &args[4..].to_vec();

        return concat(input_files, output, unprotect);
    }

    if "concat_dir" == command {
        if args.len() < 5 {
            eprintln!("Usage: {} {} <input_dir> <output.tmx> <unprotect: true|false>", args[0], command);
            std::process::exit(1);
        }

        let unprotect = match args[4].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!("unprotect must be 'true' or 'false'. Got '{}'", args[4]);
                std::process::exit(1);
            }
        };

        return concat_dir(&args[2], &args[3], unprotect);
    }

    if "filter" == command {
        if args.len() != 8 {
            eprintln!("Usage: {} {} <input.tmx> <output.tmx> <skipAuthor: true|false> <skipDocument: true|false> <skipContext: true|false>", args[0], command);
            std::process::exit(1);
        }

        let skip_author = match args[4].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!("skipAuthor must be 'true' or 'false'. Got '{}'", args[4]);
                std::process::exit(1);
            }
        };

        let skip_document = match args[5].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!("skipDocument must be 'true' or 'false'. Got '{}'", args[5]);
                std::process::exit(1);
            }
        };

        let skip_context = match args[6].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!("skipContext must be 'true' or 'false'. Got '{}'", args[6]);
                std::process::exit(1);
            }
        };

        let keep_diff_targets = match args[7].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!("keepDiffTargets must be 'true' or 'false'. Got '{}'", args[7]);
                std::process::exit(1);
            }
        };

        let skip_options = filter::SkipOptions {
            skip_author: skip_author,
            skip_document: skip_document,
            skip_context: skip_context,
            keep_diff_targets: keep_diff_targets,
        };

        return filter::filter(&args[2], &args[3], skip_options);
    }

    eprintln!("No command specified.");
    eprintln!("Usage:");
    eprintln!("  {} trim <input.tmx> <output.tmx> <N>", args[0]);
    eprintln!("  {} concat <output.tmx> <unprotect: true|false> <input1.tmx> [<input2.tmx> ...]", args[0]);
    eprintln!("  {} filter <input.tmx> <output.tmx> <skipAuthor: true|false> <skipDocument: true|false> <skipContext: true|false> <keepDiffTargets: true|false>", args[0]);

    Ok(())
}
