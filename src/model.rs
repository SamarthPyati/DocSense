use std::{
    path::{Path, PathBuf}
};

use std::collections::HashMap;

use super::lexer::*;

// Associative types
pub type FreqTable = HashMap::<String, usize>;
pub type FreqTableIndex = HashMap::<PathBuf, FreqTable>;


pub fn tf(term: &str, freq_table: &FreqTable) -> f32 {
    let n = *freq_table.get(term).unwrap_or(&0) as f32;
    // NOTE: Can lead to division by 0 if term is not in FreqTable
    // Workaround:  -> (So add 1 to denominator to prevent that (Getting negative values => REJECTED))
    //              -> Take either max of denom or 1 => APPROVED
    let d = freq_table.iter().map(|(_, c)| *c).sum::<usize>().max(1) as f32;   
    n / d
}


pub fn idf(term: &str, index: &FreqTableIndex) -> f32 {
    let n = index.len() as f32;
    // NOTE: Can lead to division by 0 if term is not in Document Corpus
    let d  = index.values().filter(|ft| ft.contains_key(term)).count().max(1) as f32;
    f32::log10(n / d)
}

pub fn search_query<'a>(query: &'a [char], tf_index: &'a FreqTableIndex) -> Vec<(&'a Path, f32)>{    
    let mut results = Vec::<(&Path, f32)>::new();
    // Cache all the tokens and don't retokenize on each query 
    let tokens = Lexer::new(&query).collect::<Vec<_>>();
    for (doc, ft) in tf_index {
        let mut rank = 0f32;   
        for token in &tokens {
            // Rank is value of tf-idf => tf * idf
            rank += tf(&token, ft) * idf(&token, tf_index);
        }
        results.push((doc, rank));
    }

    // Rank the files in desc order
    results.sort_by(|(_, ra), (_, rb)| ra.partial_cmp(rb).expect("Compared with NaN values"));
    results.reverse();
    return results;
}

