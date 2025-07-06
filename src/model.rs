use std::{
    path::{Path, PathBuf}, time::SystemTime
};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::default::Default;
use super::lexer::*;

pub trait Model {
    fn search_query(&self, query: &[char]) -> Result<Vec<(PathBuf, f32)>, ()>;
    fn add_document(&mut self, path: PathBuf, content: &[char], last_modified: SystemTime) -> Result<(), ()>;
    fn requires_reindexing(&mut self, path: &Path, last_modified: SystemTime) -> Result<bool, ()>;
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

        // Update the Docs table
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