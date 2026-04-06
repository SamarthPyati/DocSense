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
    ft: FreqTable,                  // Frequency table mapping each term to the number of times it appears within this document
    last_modified: SystemTime       // The last time this document was modified on disk. Used to detect outdated indexes and trigger reindexing when needed.
}

pub type Docs = HashMap::<PathBuf, Doc>;

#[derive(Default, Deserialize, Serialize)]
pub struct InMemoryModel {
    pub gtf: GlobalTermFreq,
    pub docs: Docs,
    // Cached sum of all doc.count values. Kept in sync by add_document /
    // remove_document so that avgdl can be computed in O(1) at query time.
    pub total_tokens: usize,
}
fn compute_avgdl(model: &InMemoryModel) -> f32 {
    if model.docs.is_empty() { return 0.0; }
    model.total_tokens as f32 / model.docs.len() as f32
}

fn compute_idf(term: &str, model: &InMemoryModel) -> f32 {
    // Number of documents in corpus
    let total_docs = model.docs.len() as f32;                           

    // Number of documents containing the unique 'term'
    let doc_freq = model.docs.values().filter(|doc| doc.ft.contains_key(term)).count() as f32;

    f32::ln(((total_docs - doc_freq + 0.5) + 1f32) / (doc_freq + 0.5))
}

/// Levenshtein edit distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 { return n; }
    if n == 0 { return m; }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i-1] == b[j-1] { 0 } else { 1 };
            curr[j] = (curr[j-1]+1).min(prev[j]+1).min(prev[j-1]+cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Expands a single stemmed/uppercased query token into a list of `(indexed_term, weight)`
/// pairs drawn from the corpus's GTF, using:
///   - Exact match            → weight 1.0
///   - Prefix overlap (≥4 ch) → weight ∝ overlap ratio × 0.85
///   - Levenshtein distance   → weight ∝ similarity × 0.75
///
/// Tokens shorter than 4 chars only allow exact matches to avoid noisy expansion.
fn expand_query_token(query_token: &str, gtf: &GlobalTermFreq) -> Vec<(String, f32)> {
    let qlen = query_token.len();
    let max_dist: usize = match qlen {
        0..=3 => 0,
        4..=5 => 1,
        6..=7 => 1,
        _ => 2,
    };

    let mut matches: HashMap<String, f32> = HashMap::new();

    for term in gtf.keys() {
        let tlen = term.len();

        // Exact match
        if term.as_str() == query_token {
            matches.insert(term.clone(), 1.0);
            continue;
        }

        if max_dist == 0 { continue; }

        // Prefix match: one token is a prefix of the other (min 4 chars)
        if qlen >= 4 && tlen >= 4 {
            if term.starts_with(query_token) || query_token.starts_with(term.as_str()) {
                let shorter = qlen.min(tlen) as f32;
                let longer  = qlen.max(tlen) as f32;
                let weight  = (shorter / longer) * 0.85;
                if weight >= 0.5 {
                    matches.entry(term.clone())
                        .and_modify(|w| *w = w.max(weight))
                        .or_insert(weight);
                    continue;
                }
            }
        }

        // Levenshtein: skip pairs whose length difference already exceeds the budget
        if qlen.abs_diff(tlen) > max_dist { continue; }
        let dist = levenshtein_distance(query_token, term);
        if dist > 0 && dist <= max_dist {
            let similarity = 1.0 - (dist as f32 / qlen.max(tlen) as f32);
            let weight = similarity * 0.75;
            matches.entry(term.clone())
                .and_modify(|w| *w = w.max(weight))
                .or_insert(weight);
        }
    }

    matches.into_iter().collect()
}

// K and B are free parameters, usually chosen, in absence of an advanced optimization, as K = [1.2, 2.0] and B = 0.75
const K: f32 = 2.0;
const B: f32 = 0.75;
/// avgdl is pre-computed once per query. Each (term, weight) pair's contribution
/// is scaled by `weight`, allowing fuzzy-matched tokens to contribute less than exact ones.
fn bm25_score(query: &[(String, f32)], doc: &Doc, model: &InMemoryModel, avgdl: f32) -> f32 {
    // Ranking documents according to BM25 Algorithm: https://en.wikipedia.org/wiki/Okapi_BM25
    if avgdl == 0.0 { return 0.0; }  // guard: no tokens in corpus means undefined avgdl
    let mut score = 0f32;
    let doc_length = doc.count as f32;

    for (term, weight) in query {
        let tf = doc.ft.get(term.as_str()).copied().unwrap_or(0) as f32;
        let idf = compute_idf(term, model);
        let denom = tf + K * (1f32 - B + B * doc_length / avgdl);
        if denom == 0.0 { continue; }
        score += weight * idf * tf * (K + 1f32) / denom;
    }
    score
}

// For TF-IDF Ranking
fn tf(term: &str, doc: &Doc) -> f32 {
    if doc.count == 0 { return 0.0; }  // guard: doc had no surviving tokens
    let n = doc.ft.get(term).cloned().unwrap_or(0) as f32;  // occurrences in this doc
    let d = doc.count as f32;                               // total tokens in this doc
    n / d
}
// For TF-IDF Ranking
fn idf(term: &str, model: &InMemoryModel) -> f32 {
    let n = model.docs.len() as f32;  // total docs in corpus
    if n == 0.0 { return 0.0; }       // guard: empty corpus → no meaningful IDF
    let d = model.gtf.get(term).cloned().unwrap_or(1) as f32;  // docs containing term
    f32::log10(n / d)
}

impl InMemoryModel {
    pub fn remove_document(&mut self, file_path: &Path) {
        if let Some(doc) = self.docs.remove(file_path) {
            // Keep the cached total in sync
            self.total_tokens = self.total_tokens.saturating_sub(doc.count);
            for term in doc.ft.keys() {
                // Update the GlobalTermFrequency table
                if let Some(freq) = self.gtf.get_mut(term) {
                    *freq = freq.saturating_sub(1);
                }
            }
        }
    }
}

use crate::RankMethod;
impl Model for InMemoryModel {
    fn search_query(&self, query: &[char], model: &InMemoryModel, rank_method: RankMethod) -> Result<Vec<(PathBuf, f32)>, ()> {
        let tokens = Lexer::new(query).collect::<Vec<_>>();

        // Expand each query token into (indexed_term, weight) pairs via exact,
        // prefix, and Levenshtein fuzzy matching. If the same indexed term is
        // reached through multiple paths, keep the highest weight.
        let mut token_weights: HashMap<String, f32> = HashMap::new();
        for token in &tokens {
            for (matched_term, weight) in expand_query_token(token, &self.gtf) {
                token_weights
                    .entry(matched_term)
                    .and_modify(|w| *w = w.max(weight))
                    .or_insert(weight);
            }
        }
        let expanded: Vec<(String, f32)> = token_weights.into_iter().collect();

        // Compute avgdl once per query (O(1) with cached total_tokens).
        let avgdl = compute_avgdl(self);

        let mut results = Vec::with_capacity(self.docs.len());
        for (path, doc) in &self.docs {
            let rank = if rank_method == RankMethod::Bm25 {
                // BM-25 Ranking — weighted fuzzy tokens, avgdl pre-computed
                bm25_score(&expanded, doc, model, avgdl)
            } else {
                // TF-IDF Ranking — weighted by fuzzy match quality
                expanded.iter()
                    .map(|(token, weight)| tf(token, doc) * idf(token, model) * weight)
                    .sum()
            };
            results.push((path.to_owned(), rank));
        }

        // partial_cmp returns None only for NaN; treat NaN as equal rather than panicking.
        results.sort_by(|(_, ra), (_, rb)| rb.partial_cmp(ra).unwrap_or(std::cmp::Ordering::Equal));
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
        let term_count: usize = ft.values().sum();

        // Skip documents with no surviving tokens (e.g. all content was stop words).
        // Indexing them would give doc.count=0, causing tf()=0/0=NaN at query time.
        if term_count == 0 {
            return Ok(());
        }

        // Update global term frequency
        for term in ft.keys() {
            self.gtf.entry(term.to_owned()).and_modify(|x| *x += 1).or_insert(1);
        }

        // Keep the cached total in sync
        self.total_tokens += term_count;

        // Update the Docs table
        self.docs.insert(file_path, Doc { count: term_count, ft, last_modified });
        Ok(())
    }

    fn requires_reindexing(&mut self, file_path: &Path, last_modified: SystemTime) -> Result<bool, ()> {
        if let Some(doc) = self.docs.get(file_path) {
            return Ok(doc.last_modified < last_modified);
        }
        return Ok(true);
    }
}

// TODO: Implement an efficient sqlite Model with parallel processing support