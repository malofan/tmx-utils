# tmx_trimmer — Fast reference (utility functionality)

Short, focused reference for the CLI utilities implemented in this repository.

## What it does
- trim: Skip the first N `<tu>` (translation unit) elements in a TMX file and write the remainder to a new file.
- concat: Merge multiple TMX files by appending their contents sequentially (no XML validation or deduplication).

## Build
- Build release binary:
    cargo build --release
- Build on MacOS:
    TARGET_CC=x86_64-linux-musl-gcc cargo build --release --target=x86_64-unknown-linux-musl
- Run locally for testing:
    cargo run -- <args>

## CLI usage
- Trim
    - Command:
        tmx-utils trim <input.tmx> <output.tmx> <N>
    - Behavior:
        - Skips the first N `<tu>` elements in `input.tmx`.
        - Writes remaining XML to `output.tmx`.
        - Carefully avoids writing stray whitespace left by skipped elements.
        - Errors surfaced via anyhow (exit on non‑zero).

- Concat
    - Command:
        tmx-utils concat <output.tmx> <input1.tmx> <input2.tmx> ...
    - Behavior:
        - Appends the contents of each input file into `output.tmx` in order.
        - No XML structure merging, validation, or deduplication performed.

- Concat directory:
    - Command:
        tmx-utils concat-dir <output.tmx> <input_directory>
    - Behavior:
        - Reads all `.tmx` files from `input_directory`.
        - Appends their contents into `output.tmx` in alphanumeric order.

- Filter:
    - Command:
        tmx-utils filter <input.tmx> <output.tmx> <skipAuthor: true|false> <skipDocument: true|false> <skipContext: true|false> 
    - Behavior:
        - Retains only `<tuv>` elements matching the specified set of options.
        - Writes filtered XML to `output.tmx`.

## Examples
- Trim first 100 units:
    cargo run -- trim big.tmx trimmed.tmx 100
- Concatenate files:
    cargo run -- concat merged.tmx part1.tmx part2.tmx part3.tmx
