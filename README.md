# DocSense: A Simple Document Search Engine

---

## Overview

DocSense is a lightweight, command-line document search engine built with Rust. It's designed to index a corpus of XML/XHTML documents, calculate term frequency-inverse document frequency (TF-IDF) scores for keywords, and serve search queries via a local HTTP server. This project provides a practical example of text processing, indexing, and basic web serving in Rust.

---

## Features

* **Document Indexing:** Indexes XML and XHTML files from a specified directory, recursively processing subdirectories.
* **TF-IDF Ranking:** Utilizes the TF-IDF algorithm to rank documents by relevance to a given search query.
* **Simple Lexer:** Custom lexer for tokenizing document content and search queries.
* **Persistent Index:** Saves the generated index to a JSON file for quick loading.
* **HTTP Server:** Provides a basic web interface for searching, accessible via your browser.

---

## Getting Started

### Prerequisites

To build and run DocSense, you'll need:

* [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)

### Building the Project

Navigate to the project's root directory and compile the application:

```bash
cargo build --release
````

The executable will be located at `target/release/DocSense`. You can also run it directly using `cargo run`.

-----

### Usage

DocSense offers three main subcommands: `index`, `check`, and `serve`.

#### 1\. `index` - Indexing Your Documents

This command indexes a specified folder containing your XML/XHTML documents and saves the generated TF-IDF index to a JSON file.

**Syntax:**

```bash
./target/release/DocSense index <folder_path> [output_index_file.json]
```

  * `<folder_path>`: The directory containing the documents to be indexed.
  * `[output_index_file.json]` (Optional): The path where the index will be saved. Defaults to `index.json`.

**Example:**

```bash
./target/release/DocSense index ./docs_corpus my_docs_index.json
```

This will index all `.xml` and `.xhtml` files within `./docs_corpus` and its subdirectories, saving the index to `my_docs_index.json`.

#### 2\. `check` - Inspecting the Index

This command allows you to quickly check how many documents are present in a saved index file.

**Syntax:**

```bash
DocSense check [input_index_file.json]
```

  * `[input_index_file.json]` (Optional): The path to the index file to check. Defaults to `index.json`.

**Example:**

```bash
./target/release/DocSense check my_docs_index.json
```

#### 3\. `serve` - Running the Search Server

This command starts an HTTP server that allows you to submit search queries and receive ranked results based on a pre-built index.

**Syntax:**

```bash
DocSense serve <input_index_file.json> [address]
```

  * `<input_index_file.json>`: The path to the index file you want to use for searching.
  * `[address]` (Optional): The IP address and port to bind the server to (e.g., `127.0.0.1:6969`). Defaults to `127.0.0.1:6969`.

**Example:**

```bash
./target/release/DocSense serve my_docs_index.json 0.0.0.0:8000
```

After starting the server, open your web browser and navigate to the specified address (e.g., `http://localhost:8000/` or `http://127.0.0.1:6969/`) to access the search interface.

-----
<!-- 
## 🤝 Contributing

Contributions, issues, and feature requests are welcome\! Feel free to check the [issues page](https://www.google.com/search?q=link_to_issues_page_if_applicable).

-----

## 📜 License

This project is licensed under the [MIT License](https://www.google.com/search?q=LICENSE) - see the `LICENSE` file for details.

```
``` -->