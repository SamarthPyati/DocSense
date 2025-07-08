# 📄 DocSense: A Simple Document Search Engine

DocSense is a lightweight, search engine built in Rust. It is designed to index and search into a large corpus of XML/XHTML/PDF/TXT/MD documents using TF-IDF or BM25 ranking. It also serves a local web interface for querying.

---

## Features

- Recursive document indexing
- TF-IDF / BM25 ranking algorithms
- Fast tokenization via custom lexer
- Persistent JSON index storage
- Local HTTP web interface

---

## Getting Started

### Build

```bash
cargo build --release
````

---

## 🧑‍💻 Usage

```bash
./target/release/DocSense <SUBCOMMAND> [OPTIONS]
```

### 🔍 `index <folder> [output.json]`

Indexes supported files (`.xml`, `.xhtml`, `.pdf`, `.txt`, `.md`) and saves index.

```bash
./DocSense index ./docs my_index.json
```

### 📦 `check [index.json]`

Prints how many documents are indexed.

```bash
./DocSense check my_index.json
```

### 🌐 `serve <index.json> [address] --rank-method <bm25|tfidf>`

Starts a local search server with ranking.

```bash
./DocSense serve my_index.json 127.0.0.1:8000 --rank-method bm25
```

Then visit [http://localhost:8000](http://localhost:8000) in your browser.

---

## 📂 Supported Formats

* XML / XHTML
* TXT / Markdown
* PDF (via Poppler)

---

## 📃 License

MIT – see [LICENSE](LICENSE) file.