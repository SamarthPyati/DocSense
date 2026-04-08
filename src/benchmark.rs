use std:: {
    fs,
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use colored::Colorize;

use crate::{RankMethod, index_directory, model::{InMemoryModel, Model}};

// Benchmark function logic
pub fn calculate_dir_size(dir_path: &Path) -> io::Result<u64> {
    let mut total_size = 0;
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total_size += calculate_dir_size(&path)?;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    }
    Ok(total_size)
}

pub fn run_benchmark(dir_path: &Path) -> Result<(), ()> {
    println!("\n{}: Starting benchmarks on '{}'", "INFO".cyan(), dir_path.display());
    
    // 1. Storage Efficiency
    let raw_size = calculate_dir_size(dir_path).unwrap_or(0);
    println!("Raw Corpus Size: {:.2} MB", raw_size as f64 / 1_048_576.0);
    
    // 2. Indexing Throughput
    let model = Arc::new(Mutex::<InMemoryModel>::new(Default::default()));
    println!("{}: Indexing directory (this may take a while)...", "INFO".cyan());
    let start_time = std::time::Instant::now();
    
    let mut index_path = dir_path.to_path_buf();
    index_path.push(".docsense.benchmark.json");
    let index_str = index_path.to_str().unwrap().to_string();
    
    index_directory(dir_path, Arc::clone(&model), Some(&index_str))?;
    
    let indexing_duration = start_time.elapsed();
    let index_size = fs::metadata(&index_path).map(|m| m.len()).unwrap_or(0);
    
    println!("Indexing Time: {:?}", indexing_duration);
    println!("Index File Size: {:.2} MB", index_size as f64 / 1_048_576.0);
    println!("Space Reduction Ratio: {:.2}x\n", raw_size as f64 / index_size.max(1) as f64);
    
    // 3. Query Latency
    println!("{}: Running query latency tests...", "INFO".cyan());
    
    let queries = vec![
        "opengl",
        "texture array shader",
        "missingkeywordthatdoesnotexist",
    ];

    let model_lock = model.lock().unwrap();

    for rank_method in vec![RankMethod::Tfidf, RankMethod::Bm25] {
        println!("\nRanking Method: {:?}", rank_method);
        println!("{:<30} | {:<15} | {:<15}", "Query", "Avg Latency", "Top Result Score");
        println!("{:-<66}", "");

        for query_str in &queries {
            let query = query_str.chars().collect::<Vec<char>>();
            
            // Warm up
            let _ = model_lock.search_query(&query, &model_lock, rank_method.clone());
            
            let iters = 10;
            let mut total_duration = std::time::Duration::new(0, 0);
            let mut top_score = 0.0;
            
            for _ in 0..iters {
                let start_query = std::time::Instant::now();
                let results = model_lock.search_query(&query, &model_lock, rank_method.clone()).unwrap_or_default();
                total_duration += start_query.elapsed();
                
                if let Some((_, score)) = results.first() {
                    top_score = *score;
                }
            }
            
            let avg_duration = total_duration / iters;
            println!("{:<30} | {:<15?} | {:.4}", query_str, avg_duration, top_score);
        }
    }
    
    println!("\n{}: Benchmark complete.", "INFO".cyan());
    Ok(())
}