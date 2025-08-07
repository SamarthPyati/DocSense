use clap::{Subcommand, command, Parser}; 
use crate::RankMethod;

#[derive(Parser)]
#[command(name = "DocSense", version, author, about, long_about = None)]
#[command(about = "A fast document indexing and search engine which runs locally on your machine.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(
        about = "Search for a query string in an indexed file",
        long_about = "Search for a prompt string using BM25 (or TF-IDF) ranking algorithm across previously indexed documents."
    )]
    Search {
        #[arg(help = "Path to the .json index file (e.g., index.docsense.json)")]
        index_file_path: String, 
        #[arg(help = "Search prompt string (e.g., 'deep neural networks')")]
        prompt: String, 
        #[arg(short, long, default_value = "tfidf", value_enum, help = "Ranking algorithm to use")]
        rank_method: RankMethod,
    }, 

    #[command(
        about = "Check how many files are indexed",
        long_about = "Display number of documents currently indexed in the specified index file. Useful for verifying the index state."
    )]
    Check {
        #[arg(default_value="index.json", help="Path to index file to inspect")]
        index_file_path: String, 
    }, 

    #[command(
        about = "Serve directory over HTTP with search interface",
        long_about = "Indexes the provided directory recursively and starts a web server for querying indexed files through a UI."
    )]
    Serve {
        #[arg(help = "Path to directory to index and serve")]
        dir_path: String, 
        #[arg(default_value = "127.0.0.1:6969", help = "IP:PORT to bind HTTP server (e.g., 0.0.0.0:8080)")]
        address: String, 
        #[arg(short, long, default_value = "tfidf", value_enum, help = "Ranking algorithm to use")]
        rank_method: RankMethod,
    }
}