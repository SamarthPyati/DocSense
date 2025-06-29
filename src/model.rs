use std::{
    path::{self, Path, PathBuf}, time::SystemTime
};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::default::Default;
use super::lexer::*;

use sqlite::{self};
use colored::{Colorize};

pub trait Model {
    fn search_query(&self, query: &[char]) -> Result<Vec<(PathBuf, f32)>, ()>;
    fn add_document(&mut self, path: PathBuf, content: &[char], last_modified: SystemTime) -> Result<(), ()>;
    fn requires_reindexing(&mut self, path: &Path, last_modified: SystemTime) -> Result<bool, ()>;
}

pub struct SqliteModel {
    connection: sqlite::Connection
}

impl SqliteModel {
    pub fn execute(&self, statement: &str) -> Result<(), ()> {
        self.connection.execute(statement).map_err(|err| {
            eprintln!("{}: Failed to execute query {query} as {err}", "ERROR".bold().red(), query = statement.bright_blue(), err = err.to_string().red());
        })?;
        Ok(())
    }

    pub fn begin(&self) -> Result<(), ()> {
        // TODO: Add error logging
        self.execute("BEGIN;")
    }
    
    pub fn commit(&self) -> Result<(), ()> {
        // TODO: Add error logging
        self.execute("COMMIT;")
    }
    
    pub fn open(path: &Path) -> Result<Self, ()> {
        let connection = sqlite::open(path).map_err(|err| {
            eprintln!("{}: Could not open sqlite database {path} as {err}", "ERROR".bold().red(), path = path.display().to_string().bright_blue(), err = err.to_string().red());
        })?;

        let this = Self {connection};

        // Table Documents (Contains path and term_count)
        this.execute("
            CREATE TABLE IF NOT EXISTS Documents (
                id INTEGER NOT NULL PRIMARY KEY,
                path TEXT,
                term_count INTEGER,
                UNIQUE(path)
            );
        ")?;

        // Table FreqTable (Contains map of term with its count refering to a document)
        this.execute("
            CREATE TABLE IF NOT EXISTS FreqTable (
                term TEXT,
                doc_id INTEGER,
                freq INTEGER,
                UNIQUE(term, doc_id),
                FOREIGN KEY(doc_id) REFERENCES Documents(id)
            );
       ")?;

        // Table GlobalTermFreq (Contains map of term with its frequency of occurence in entire document corpus)
        this.execute("
            CREATE TABLE IF NOT EXISTS GlobalTermFreq (
                term TEXT,
                freq INTEGER,
                UNIQUE(term)
            );
        ")?;

        Ok(this)
    }

    pub fn check(&self) -> Result<u64, ()> {
        let count = {
            let query = "SELECT COUNT(*) as count FROM Documents";
            let log_err = |err: sqlite::Error| {
                eprintln!("{ERROR}: Could not execute query {query} as {err}", ERROR = "ERROR".bold().red(), err = err.to_string().red());
            };

            let mut stmt = self.connection.prepare(query).map_err(log_err)?;   
            match stmt.next().map_err(log_err)? {
                sqlite::State::Row => stmt.read::<i64, _>("count").map_err(log_err)?, 
                sqlite::State::Done => 0, 
            }
        };
        Ok(count as u64)
    }
    
}

impl Model for SqliteModel {
    fn search_query(&self, _query: &[char]) -> Result<Vec<(PathBuf, f32)>, ()> {
        todo!("SqliteModel::search_query()");
    }

    fn requires_reindexing(&mut self, _path: &Path, _last_modified: SystemTime) -> Result<bool, ()> {
        Ok(true)
    }

    
    fn add_document(&mut self, path: PathBuf, content: &[char], _last_modified: SystemTime) -> Result<(), ()> {
        let terms = Lexer::new(content).collect::<Vec<_>>();   
        // Populate Documents Table
        let doc_id = {
            let query = "INSERT INTO Documents (path, term_count) VALUES (:path, :count)";
            
            let log_err = |err: sqlite::Error| {
                eprintln!("{ERROR}: Could not execute query {query} as {err}", ERROR = "ERROR".bold().red(), err = err.to_string().red());
            };

            let mut stmt = self.connection.prepare(query).map_err(log_err)?;
            stmt.bind_iter::<_, (_, sqlite::Value)>([
                (":path", path.display().to_string().as_str().into()),
                (":count", (terms.len() as i64).into()),
            ]).map_err(log_err)?;
            stmt.next().map_err(log_err)?;

            unsafe {
                sqlite3_sys::sqlite3_last_insert_rowid(self.connection.as_raw())
            }
        };

        let mut freq_table = FreqTable::new();

        for term in &terms {
            freq_table.entry(term.to_owned()).and_modify(|x| *x += 1).or_insert(1);
        }

        for (term, freq) in &freq_table {
            // Populate FreqTable
            {
                let query = "INSERT INTO FreqTable(doc_id, term, freq) VALUES (:doc_id, :term, :freq)";
                let log_err = |err: sqlite::Error| {
                    eprintln!("{ERROR}: Could not execute query {query} as {err}", ERROR = "ERROR".bold().red(), err = err.to_string().red());
                };

                let mut stmt = self.connection.prepare(query).map_err(log_err)?;   
                stmt.bind_iter::<_, (_, sqlite::Value)>([
                    (":doc_id", doc_id.into()),
                    (":term", term.as_str().into()),
                    (":freq", (*freq as i64).into()),
                ]).map_err(log_err)?;
                stmt.next().map_err(log_err)?;
            }

            // Populate GlobalTermFreq
            {   
                let freq = {
                    let query = "SELECT freq FROM GlobalTermFreq WHERE term = :term";
                    let log_err = |err: sqlite::Error| {
                        eprintln!("{ERROR}: Could not execute query {query} as {err}", ERROR = "ERROR".bold().red(), err = err.to_string().red());
                    };
    
                    let mut stmt = self.connection.prepare(query).map_err(log_err)?;   
                    stmt.bind_iter::<_, (_, sqlite::Value)>([
                    (":term", term.as_str().into()),
                    ]).map_err(log_err)?;
                    match stmt.next().map_err(log_err)? {
                        sqlite::State::Row => stmt.read::<i64, _>("freq").map_err(log_err)?, 
                        sqlite::State::Done => 0, 
                    }
                };

                // TODO: Find better way to autoincrement the frequency
                let query = "INSERT OR REPLACE INTO GlobalTermFreq(term, freq) VALUES (:term, :freq)";
                let log_err = |err: sqlite::Error| {
                        eprintln!("{ERROR}: Could not execute query {query} as {err}", ERROR = "ERROR".bold().red(), err = err.to_string().red());
                };
    
                let mut stmt = self.connection.prepare(query).map_err(log_err)?;   
                stmt.bind_iter::<_, (_, sqlite::Value)>([
                    (":term", term.as_str().into()),
                    (":freq", ((freq + 1) as i64).into()),
                ]).map_err(log_err)?;
                stmt.next().map_err(log_err)?;
            }
        }

        Ok(())
    }
}

// ---- Associative types ----

/* Answers how frequently a term occurs in a single document. 
   Map of term with its frequency of occurence single document. */
pub type FreqTable = HashMap::<String, usize>;  

/* PREVIOUS: Map of a document with a pair containing (frequency table, total terms in that table (i.e sum of values)). */
// pub type FreqTableIndex = HashMap::<PathBuf, (usize, FreqTable)>;

/* Answers how frequently a term occurs in all documents. 
   Map of term with frequency of occurence in all corpus of documents.*/
pub type GlobalTermFreq = HashMap::<String, usize>;

#[derive(Serialize, Deserialize)]
pub struct Doc {
    count: usize, 
    ft: FreqTable, 
    last_modified: SystemTime
}

pub type Docs = HashMap::<PathBuf, Doc>;

#[derive(Default, Deserialize, Serialize)]
pub struct InMemoryModel {
    pub gtf: GlobalTermFreq, 
    pub docs: Docs, 
}

fn compute_tf(term: &str, doc: &Doc) -> f32 {
    let n = doc.ft.get(term).cloned().unwrap_or(0) as f32;   
    let d = doc.count as f32;
    n / d
}

fn compute_idf(term: &str, model: &InMemoryModel) -> f32 {
    let n = model.docs.len() as f32;
    // NOTE: Can lead to division by 0 if term is not in Document Corpus
    // Set Denominator to 1 if turns to 0
    let d  = model.gtf.get(term).cloned().unwrap_or(1) as f32;
    f32::log10(n / d)
}

impl InMemoryModel {
    fn remove_document(&mut self, file_path: &Path) {
        // Remove the doc from docs 
        if let Some(doc) = self.docs.remove(file_path) {
            for term in doc.ft.keys() {
                // Update the GlobalTermFrequency table
                if let Some(freq) = self.gtf.get_mut(term) {
                    *freq -= 1;
                }
            }
        }
    }
}

impl Model for InMemoryModel {
    fn search_query(&self, query: &[char]) -> Result<Vec<(PathBuf, f32)>, ()> {
        let mut results = Vec::new();
        // Cache all the tokens and don't retokenize on each query 
        let tokens = Lexer::new(&query).collect::<Vec<_>>();
        for (path, doc) in &self.docs {
            let mut rank = 0f32;   
            for token in &tokens {
                // Rank is value of tf-idf => tf * idf
                rank += compute_tf(&token, &doc) * compute_idf(&token, &self);
            }
            results.push((path.to_owned(), rank));
        }

        // Rank the files in desc order
        results.sort_by(|(_, ra), (_, rb)| ra.partial_cmp(rb).expect("Compared with NaN values"));
        results.reverse();
        Ok(results)
    }

    fn add_document(&mut self, file_path: PathBuf, content: &[char], last_modified: SystemTime) -> Result<(), ()> {
        // Remove earlier document
        self.remove_document(&file_path);

        // Precompute all the tokens at once 
        let tokens = Lexer::new(&content).collect::<Vec<_>>();
        let mut ft = FreqTable::new();
        for token in &tokens {
            ft.entry(token.to_owned()).and_modify(|x| *x += 1).or_insert(1);
        }
        
        // Total count of terms in FreqTable 
        let term_count = ft.iter().map(|(_, c)| *c).sum();

        // Update global term frequency
        for term in ft.keys() {
            self.gtf.entry(term.to_owned()).and_modify(|x| *x += 1).or_insert(1);
        }
        self.docs.insert(file_path, Doc { count: term_count, ft: ft , last_modified: last_modified});
        Ok(())
    }

    fn requires_reindexing(&mut self, file_path: &Path, last_modified: SystemTime) -> Result<bool, ()> {
        if let Some(doc) = self.docs.get(file_path) {
            return Ok(doc.last_modified < last_modified);
        }
        return Ok(true);
    }

}

// TODO: Implement a efficient sqlite Model with parellel processing support