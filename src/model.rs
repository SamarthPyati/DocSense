use std::{
    path::{Path, PathBuf}, time::SystemTime
};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::default::Default;

use super::lexer::*;

pub trait Model {
    fn search_query(&self, query: &[char], model: &InMemoryModel, rank_method: RankMethod) -> Result<Vec<(PathBuf, f32)>, ()>;
    fn add_document(&mut self, path: PathBuf, content: &[char], last_modified: SystemTime) -> Result<(), ()>;
    fn requires_reindexing(&mut self, path: &Path, last_modified: SystemTime) -> Result<bool, ()>;
}

// ---- Associative types ----
/* Answers how frequently a term occurs in a single document. 
   Map of term with its frequency of occurence single document. */
pub type FreqTable = HashMap::<String, usize>;  

/* Answers how frequently a term occurs in all documents. 
   Map of term with frequency of occurence in all corpus of documents. */
pub type GlobalTermFreq = HashMap::<String, usize>;

#[derive(Serialize, Deserialize)]
pub struct Doc {
    count: usize,                   // Total number of terms (tokens) present in this document.
    ft: FreqTable,                  //  Frequency table mapping each term to the number of times it appears within this document
    last_modified: SystemTime       // The last time this document was modified on disk. Used to detect outdated indexes and trigger reindexing when needed.
}

pub type Docs = HashMap::<PathBuf, Doc>;

#[derive(Default, Deserialize, Serialize)]
pub struct InMemoryModel {
    pub gtf: GlobalTermFreq, 
    pub docs: Docs, 
}

fn compute_avgdl(model: &InMemoryModel) -> f32 {
    let total: usize = model.docs.values().map(|doc| doc.count).sum();
    total as f32 / model.docs.len() as f32
}

fn compute_idf(term: &str, model: &InMemoryModel) -> f32 {
    // Number of documents in corpus
    let total_docs = model.docs.len() as f32;                           

    // Number of documents containing the unique 'term'
    let doc_freq = model.docs.values().filter(|doc| doc.ft.contains_key(term)).count() as f32;

    f32::ln(((total_docs - doc_freq + 0.5) + 1f32) / (doc_freq + 0.5))
}

// K and B are free parameters, usually chosen, in absence of an advanced optimization, as K = [1.2, 2.0] and B = 0.75
const K: f32 = 2.0;
const B: f32 = 0.75;
fn bm25_score(query: &Vec<String>, doc: &Doc, model: &InMemoryModel) -> f32 {
    // Ranking documents according to BM25 Algorithm: https://en.wikipedia.org/wiki/Okapi_BM25
    let mut score= 0f32;
    let avgdl = compute_avgdl(model);
    let doc_length = doc.count as f32;

    for term in query {
        // Number of times a term occurs in the document
        let tf = doc.ft.get(term).copied().unwrap_or(0) as f32;
        let idf = compute_idf(&term, model);

        let denom = tf + K * (1f32 - B + B * doc_length / avgdl);
        score += idf * tf * (K + 1f32) / denom;
    }
    score
}

// For TF-IDF Ranking 
fn tf(term: &str, doc: &Doc) -> f32 {
    let n = doc.ft.get(term).cloned().unwrap_or(0) as f32;     // Number of times term occured in document
    let d = doc.count as f32;                                           // Total number of terms present in document 
    n / d   
}
// For TF-IDF Ranking
fn idf(term: &str, model: &InMemoryModel) -> f32 {
    let n = model.docs.len() as f32;
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
                    *freq -= freq.saturating_sub(1);
                }
            }
        }
    }
}

use crate::RankMethod;
impl Model for InMemoryModel {
    fn search_query(&self, query: &[char], model: &InMemoryModel, rank_method: RankMethod) -> Result<Vec<(PathBuf, f32)>, ()> {
        let tokens = Lexer::new(&query).collect::<Vec<_>>();
        
        let mut results = Vec::with_capacity(self.docs.len());
        for (path, doc) in &self.docs {
            let rank = if rank_method == RankMethod::Bm25 {
                // BM-25 Ranking
                bm25_score(&tokens, doc, model)
            } else {
                // TF-IDF Ranking
                tokens.iter()
                    .map(|token| tf(token, doc) * idf(token, model))
                    .sum()
            };

            results.push((path.to_owned(), rank));
        }

        // Rank the files in desc order
        results.sort_by(|(_, ra), (_, rb)| rb.partial_cmp(ra).expect("Compared with NaN values"));
        Ok(results)
    }

    fn add_document(&mut self, file_path: PathBuf, content: &[char], last_modified: SystemTime) -> Result<(), ()> {
        // Remove earlier document
        self.remove_document(&file_path);

        // Precompute all the tokens at once 
        let tokens = Lexer::new(&content).collect::<Vec<_>>();
        let mut ft = FreqTable::new();
        for token in &tokens {
            ft.entry(token.clone()).and_modify(|x| *x += 1).or_insert(1);
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

// TODO: BM25 is very slow, optimize it 
// TODO: Implement a efficient sqlite Model with parellel processing support