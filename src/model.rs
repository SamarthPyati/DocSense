use std::{
    path::{Path, PathBuf}
};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::lexer::*;

// ---- Associative types ----

/* Answers how frequently a term occurs in a single document. 
   Map of term with its frequency of occurence single document. */
pub type FreqTable = HashMap::<String, usize>;  

/* Map of a document with its frequency table. */
pub type FreqTableIndex = HashMap::<PathBuf, FreqTable>;

/* Answers how frequently a term occurs in all documents. 
   Map of term with frequency of occurence in all corpus of documents.*/
pub type GlobalTermFreq = HashMap::<String, usize>;

#[derive(Default, Deserialize, Serialize)]
pub struct Model {
    pub gtf: GlobalTermFreq, 
    pub tf_index: FreqTableIndex
}

pub fn compute_tf(term: &str, freq_table: &FreqTable) -> f32 {
    let n = *freq_table.get(term).unwrap_or(&0) as f32;
    // NOTE: Can lead to division by 0 if term is not in FreqTable
    // Workaround:  -> (So add 1 to denominator to prevent that (Getting negative values => REJECTED))
    //              -> Take either max of denom or 1 => APPROVED
    let d = freq_table.iter().map(|(_, c)| *c).sum::<usize>().max(1) as f32;   
    n / d
}


pub fn compute_idf(term: &str, model: &Model) -> f32 {
    let n = model.tf_index.len() as f32;
    // NOTE: Can lead to division by 0 if term is not in Document Corpus
    // Set Denominator to 1 if turns to 0
    let d  = *model.gtf.get(term).unwrap_or(&1) as f32;
    f32::log10(n / d)
}

pub fn search_query<'a>(query: &'a [char], model: &'a Model) -> Vec<(&'a Path, f32)>{    
    let mut results = Vec::<(&Path, f32)>::new();
    // Cache all the tokens and don't retokenize on each query 
    let tokens = Lexer::new(&query).collect::<Vec<_>>();
    for (doc, ft) in &model.tf_index {
        let mut rank = 0f32;   
        for token in &tokens {
            // Rank is value of tf-idf => tf * idf
            rank += compute_tf(&token,&ft) * compute_idf(&token, &model);
        }
        results.push((&doc, rank));
    }

    // Rank the files in desc order
    results.sort_by(|(_, ra), (_, rb)| ra.partial_cmp(rb).expect("Compared with NaN values"));
    results.reverse();
    return results;
}

