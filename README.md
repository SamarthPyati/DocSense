# DocSense: A Fast, Local Document Search Engine

DocSense is a lightweight, search engine built in Rust. It is designed to index and search into a large corpus of XML/XHTML/PDF/TXT/MD documents using TF-IDF or BM25 ranking. It also serves a local web interface for querying.

## Features

- **Recursive Document Indexing:** Parses through deeply nested directories.
- **Dynamic Re-indexing:** Automatically cleans up deleted files and integrates new modifications incrementally.
- **Two Ranking Architectures:** 
  - **BM25 (Default):** O(N) optimized complexity with pre-cached lengths.
  - **TF-IDF:** Classic term frequency-inverse document frequency weighting.
<!-- - **Fuzzy Semantic Matching:** Implements Prefix-overlap and Levenshtein distance expansion to find partial matches or misspelled tokens (e.g. searching "neural" will match "neural network" papers). -->
- **Portable & Self-Contained Binary:** The web UI (HTML/JS/CSS) is embedded at compile-time. The server can be run from anywhere on your machine without external asset dependencies.
- **Persistent Local Index:** Automatically caches generated `.docsense.json` representations of your corpus to skip redundant re-parsing.

---

## Getting Started

### Prerequisites
- Build tools for Rust (`cargo`)
- `poppler` (Required for building the PDF parsing dependencies)
  - **macOS:** `brew install poppler`
  - **Linux:** `sudo apt-get install libpoppler-glib-dev`

### Build

```bash
cargo build --release
```
The compiled self-contained binary will be available at `./target/release/DocSense`.

---

## Command Line Usage

DocSense has a straightforward CLI allowing you to build indices offline, inspect them, or immediately start the web server.

```bash
./target/release/DocSense <SUBCOMMAND> [OPTIONS]
```

### 1. `serve` (Web Interface)

Recursively indexes the specified directory, spins up the embedded HTTP server, and hosts the visual search interface. Re-indexing (pruning deleted files, adding new ones) happens automatically on boot.

```bash
# Serves the docs folder on port 6969
./DocSense serve ./docs 127.0.0.1:6969
```
*Options:*
- `--rank-method <tfidf|bm25>`: Switch the core ranking algorithm. (Default: `tfidf`)

### 2. `index` (Offline Indexing)

Generates the `.docsense.json` index file for a directory without starting the web server. Excellent for CI/CD pipelines or cron jobs.

```bash
# Automatically saves index to ./docs/.docsense.json
./DocSense index ./docs 

# Or specify a custom output target
./DocSense index ./docs path/to/my_index.json
```

### 3. `search` (CLI Search)

Perform a search directly from the terminal against a pre-built index file.

```bash
./DocSense search ./docs/.docsense.json "attention networks" --rank-method bm25
```

### 4. `check` (Index Stats)

Inspect a compiled JSON index to see the total number of processed entries.

```bash
./DocSense check ./docs/.docsense.json
```

---

## Supported Formats

- `.txt` / `.md` (Raw text extraction)
- `.xml` / `.xhtml` (Markup stripped parsing)
- `.pdf` (Parsed natively via Poppler)

## License

GPL - see [LICENSE](LICENSE) file.