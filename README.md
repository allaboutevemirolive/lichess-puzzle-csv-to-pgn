# Lichess Puzzle CSV to PGN

A native Rust application that filters an unpacked [Lichess puzzle database](https://database.lichess.org/#puzzles) CSV by minimum rating and exports the matching puzzles as PGN.

It replaces the old Java/JavaFX app, so no Java Runtime Environment is needed. The legacy Java source remains in [`lichess/`](lichess/) only for historical reference.

## What it does

- Reads the official Lichess CSV format, including properly quoted CSV fields.
- Includes puzzles with a rating **greater than or equal to** the minimum rating.
- Validates each exported FEN and UCI move sequence.
- Converts UCI moves to standard algebraic notation (SAN), producing usable PGN move text.
- Preserves puzzle metadata (`PuzzleId`, rating, themes, popularity, opening tags, and source URL) as PGN tags.
- Streams the source and writes through a temporary file, so a failed conversion never replaces an existing output PGN.

## Requirements

- Rust 1.85 or newer. Install it with [rustup](https://rustup.rs/) if `cargo --version` is unavailable.
- An **uncompressed** Lichess puzzle CSV. The database download may arrive as `.csv.zst`; unpack it before converting.

On macOS, if needed:

```sh
brew install zstd
zstd -d lichess_db_puzzle.csv.zst
```

## Run the desktop app

From this repository:

```sh
cargo run --release
```

Choose the unpacked CSV, choose the destination PGN, enter a minimum rating, then select **Convert to PGN**. The app keeps the interface responsive while it converts and displays record counts as it runs.

To build the standalone executable without launching it:

```sh
cargo build --release
```

The executable is created at `target/release/lichess-puzzle-csv-to-pgn` (or `.exe` on Windows).

## Command line

The same converter is also available without a GUI:

```sh
cargo run --release --bin lichess-puzzle-csv-to-pgn-cli -- \
  --input lichess_db_puzzle.csv \
  --output puzzles-1500-plus.pgn \
  --min-rating 1500
```

Use `--help` to see the available flags.

## Development checks

```sh
cargo fmt --check
cargo test --all-targets
```
