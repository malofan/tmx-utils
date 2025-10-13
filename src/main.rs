use anyhow::{Context, Result};
use std::env;

mod whitespace;

mod attribute;

mod trim;
use self::trim::trim;

mod concat;
use self::concat::concat;

mod filter;

fn main() -> Result<()> {
    // Usage: tmx_trimmer <input.tmx> <output.tmx> <N>
    let args: Vec<String> = env::args().collect();

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

    eprintln!("No command specified.");
    eprintln!("Usage:");
    eprintln!("  {} trim <input.tmx> <output.tmx> <N>", args[0]);
    eprintln!("  {} concat <output.tmx> <unprotect: true|false> <input1.tmx> [<input2.tmx> ...]", args[0]);

    Ok(())
}
