use std:: {
    fs::{self, File}, io::{self, BufReader, BufWriter, Read}, path::Path, process::{exit, ExitCode}, str::{self}, sync::{Arc, Mutex}, thread
};


use clap::{Parser, Subcommand, ValueEnum};
use xml::{self, reader::XmlEvent, EventReader};
use xml::common::{TextPosition, Position};
use colored::{Colorize};

mod lexer;
mod server;
mod model;

use crate::model::*;
use poppler::{Document};

/* Parse all the text (Character Events) from the XML File */
fn parse_xml_file(file_path: &Path) -> Result<String, ()> {
    let file = File::open(file_path).map_err(|err| {
        eprintln!("{}: Could not open file {file_path}: {err}", "ERROR".bold().red(), file_path = file_path.display());
    })?;
    let er = EventReader::new(BufReader::new(file));
    let mut content = String::new();

    for event in er.into_iter() {
        let event = event.map_err(|err| {
            let TextPosition {row, column} = err.position();
            let msg = err.msg();
            eprintln!("{file_path}:{row}:{column}: {err}: {msg}", err = "ERROR".red(), 
                                                                file_path = file_path.display());
        })?;

        if let XmlEvent::Characters(text) = event {
            content.push_str(&text);
            content.push(' ');
        }
    }
    Ok(content)
}

/* Parse all the text from the TXT File */
fn parse_txt_file(file_path: &Path) -> Result<String, ()> {
    let mut content = String::new();
    let mut file = File::open(file_path).map_err(|err| {
        eprintln!("{}: Could not open file {path} as {err}", "ERROR".bold().red(), path = file_path.to_string_lossy().bright_blue(), err = err.to_string().red());
    })?;

    let _ = file.read_to_string(&mut content);
    Ok(content)
}

/* Parse all the text from the PDF File with Poppler */
fn parse_pdf_files(file_path: &Path) -> Result<String, ()> {
    let mut file_content = Vec::new();

    File::open(file_path)
        .and_then(|mut file| file.read_to_end(&mut file_content))
        .map_err(|err| {
            eprintln!("{}: Failed to get contents of PDF file {path} as {err}", "ERROR".red().bold(), path = file_path.to_string_lossy().bright_blue(), err = err.to_string().red());
        })?;


    let pdf_file = Document::from_data(&file_content, None).map_err(|err| {
        eprintln!("{}: Failed to make poppler document out of PDF file {path} as {err}", "ERROR".red().bold(), path = file_path.to_string_lossy().bright_blue(), err = err.to_string().red());
    })?;

    let mut content = String::new();
    if pdf_file.n_pages() > 0 {
        for i in 0..pdf_file.n_pages() { 
            if let Some(page) = pdf_file.page(i) {
                if let Some(text) = page.text() {
                    content.push_str(&text.as_str());
                }
            } else {
                eprintln!("{}: Could not get text of page {i} of {file_path}.", "ERROR".red().bold(), file_path = file_path.to_string_lossy().bright_blue())
            }
        }
    }
    Ok(content)
}

fn parse_file_by_ext(file_path: &Path) -> Result<String, ()> {
    let ext = file_path.extension().ok_or_else(|| {
        eprintln!("{}: Could not get extension of {path}", "ERROR".bold().red(), path = file_path.to_string_lossy().bright_blue());   
    })?.to_str();

    match ext.unwrap() {
        "xml" | "xhtml" => {
            return parse_xml_file(file_path);
        }

        "txt" | "md" => {
            return parse_txt_file(file_path); 
        }

        "pdf" => {
            return parse_pdf_files(file_path);
        }

        _ => {
            eprintln!("{}: Extension {ext} is unsupported", "ERROR".bold().red(), ext = ext.unwrap());   
            Err(())
        }
    }
}

/* Save the model to a path as json file */
fn save_model_as_json(model: &InMemoryModel, index_path: &str) -> Result<(), ()> {
    println!("Saving {} ...", index_path.bright_blue());
    
    let index_file = File::create(index_path).map_err(|err| {
        eprintln!("{}: Failed to create file {path} as {err}", "ERROR".bold().red(), path = index_path.bright_blue(), err = err.to_string().red());
    })?;

    serde_json::to_writer(BufWriter::new(index_file), &model).unwrap_or_else(|err| {
        eprintln!("{}: Failed to save file {path} as {err}", "ERROR".bold().red(), path = index_path.bright_blue(), err = err.to_string().red());
    });
    Ok(())
}

const ALLOWED_FILE_TYPE_EXTENSIONS: [&str; 5] = ["xml", "xhtml", "txt", "md", "pdf"];

/* Indexes a folder as a json file and adds to model, Processed is the number of file indexed */
fn append_folder_to_model(dir_path: &Path, model: Arc<Mutex<InMemoryModel>>, processed: &mut usize) -> Result<(), ()> {
    let dir = fs::read_dir(dir_path).map_err(|err| {
        eprintln!("{}: Failed to read directory {dir_path} as \"{err}\"", "ERROR".bold().red(), 
                                                                            dir_path = dir_path.to_str().unwrap().bold().bright_blue(), 
                                                                            err = err.to_string().red());
    })?;

    'step: for file in dir {
        let file = file.map_err(|err| {
            eprintln!("{}: Failed to read next file in directory {dir_path} as {err}", "ERROR".bold().red(), 
                                                                                        dir_path = dir_path.to_str().unwrap().bold().bright_blue(), 
                                                                                        err = err.to_string().red());
        })?;

        let file_path = file.path();
        let file_path_str = file_path.to_str().unwrap();
        
        let last_modified = file.metadata().map_err(|err| {
            eprintln!("{}: Failed to get metadata of file {path} as {err}", "ERROR".bold().red(), path = file_path_str.bright_blue(), err = err.to_string().red());
        })?.modified().map_err(|err| {
            eprintln!("{}: Failed to last modified time of file {path} as {err}", "ERROR".bold().red(), path = file_path_str.bright_blue(), err = err.to_string().red());
        }).unwrap();
        
        let file_ext = file_path.extension();

        // Skip unsupported files
        if let Some(ext) = file_ext {
            if !ALLOWED_FILE_TYPE_EXTENSIONS.contains(&ext.to_str().unwrap()) {
                println!("{}: Skipping unsupported file {}", "INFO".cyan(), file_path_str.bright_yellow());
                continue 'step;
            }
        }

        // Skip dot files or folders 
        let dot_files = file_path
                              .file_name()
                              .and_then(|s| s.to_str())
                              .map(|s| s.starts_with("."))
                              .unwrap();
        if dot_files {
            continue 'step;
        }

        // If file is another directory recursively index it too 
        let file_type = file.file_type().map_err(|err| {
            eprintln!("{err}: Failed to determine file type for {file_path} as {msg}", err = "ERROR".bold().red(), file_path = file_path_str.bold().bright_blue(), msg = err.to_string().red());
        })?;

        // Recursively index all the folders
        if file_type.is_dir() {
            append_folder_to_model(&file_path, Arc::clone(&model), processed)?;
            continue 'step;
        }   

        // Get absolute file path 
        let file_path = fs::canonicalize(&file_path).map_err(|err| {
            eprintln!("{}: Could not canonicalize path {} as {}", "ERROR".red().bold(),
            file_path.display().to_string().bright_blue(),
            err.to_string().red());
        })?;
        
        // Main 
        let mut model = model.lock().unwrap();
        if model.requires_reindexing(&file_path, last_modified)? {
            println!("{}: Indexing {} ...", "INFO".cyan(), file_path_str.bright_cyan());
    
            let content = match parse_file_by_ext(&file_path) {
                Ok(content) => content.chars().collect::<Vec<_>>(),
                Err(_) => {
                    eprintln!("{}: Failed to read xml file {path}", "ERROR".bold().red(), path = file_path.to_str().unwrap().bright_blue());
                    continue 'step;
                }
            };

            model.add_document(file_path, &content, last_modified)?;
            *processed += 1;
        }  
    }
    Ok(())
}

/* Check the amount of files present in the main frequency table index */
fn check_index(index_path: &str) -> Result<(), ()> {
    let index_file = fs::File::open(index_path).map_err(|err| {
        eprintln!("{}: Could not open file {file} as \"{err}\"", "ERROR".bold().red(), file = index_path.bright_blue(), err = err.to_string().red());
        exit(1);
    })?;

    println!("{info}: Reading file {file}", info = "INFO".cyan(), file = index_path.bright_blue());
    let model: InMemoryModel = serde_json::from_reader(BufReader::new(index_file)).map_err(|err|  {
        eprintln!("{}: Serde could not read file {file} as \"{err}\"", "ERROR".bold().red(), file = index_path.bright_blue(), err = err.to_string().red());
        exit(1);
    })?;
    println!("{info}: Index file has {entries} entries", info = "INFO".cyan(), entries = model.docs.len());
    Ok(())  
}

/* Fetch the InMemory model from an index file */
fn fetch_model(index_path: &str) -> Result<InMemoryModel, ()> {
    let index_file = fs::File::open(&index_path).map_err(|err| {
        eprintln!("{}: Could not open file {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = index_path.bright_blue(), err = err.to_string().red());
    }).unwrap();

    let model = serde_json::from_reader(BufReader::new(index_file)).map_err(|err| {
        eprintln!("{}: Serde failed to read {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = index_path.bright_blue(), err = err.to_string().red());
    }).unwrap();

    return Ok(model);
}

#[derive(Parser)]
#[command(name = "DocSense", version, author, about, long_about = None)]
#[command(about = "A fast document indexing and search engine which runs locally on your machine.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum RankMethod {
    Tfidf, 
    Bm25
}

#[derive(Subcommand)]
enum Commands {
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

fn entry() -> Result<(), ()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {index_file_path, prompt, rank_method} => {
            let prompt = prompt.chars().collect::<Vec<char>>();
            let model = fetch_model(&index_file_path)?;
            for (path, rank) in model.search_query(&prompt, &model, rank_method)?.iter().take(20) {
                println!("{path} - {rank}", path = path.display());
            } 

            return Ok(());
        }

        Commands::Check { index_file_path } => {
            check_index(&index_file_path).unwrap();
        }

        Commands::Serve { dir_path, address , rank_method} => {
            // IDEATE: Is it fine to place the index file in the folder itself or place in a root dir?
            let mut index_path = Path::new(&dir_path).to_path_buf(); 
            index_path.push(".docsense.json");

            let model: Arc<Mutex<InMemoryModel>>;
            if index_path.exists() {
                // Fetch already existing model 
                model = Arc::new(Mutex::new(fetch_model(&index_path.to_str().unwrap()).unwrap_or_else(|()| {
                    eprintln!("{}: Failed to fetch model for {}.", "ERROR".bold().red(), index_path.to_string_lossy().bright_blue());
                    exit(1);
                })));
            } else {
                // Create a new model if not present 
                model = Arc::new(Mutex::new(Default::default()));
            }
            
            {
                let model = Arc::clone(&model); 
                thread::spawn(move || {
                    let mut processed: usize = 0 as usize;
                    append_folder_to_model(Path::new(&dir_path), Arc::clone(&model), &mut processed).unwrap();
                    
                    // Save the model only when some files are processed
                    if processed > 0 {
                        let model= model.lock().unwrap();
                        save_model_as_json(&model, index_path.to_str().unwrap()).unwrap();
                    }
                    println!("{}: Finished indexing ...", "INFO".cyan());
                });
            }
            // TODO: Print the information of server start at the end of logging
            return server::start(&address, Arc::clone(&model), rank_method);
        }   
    }
    Ok(())
}

fn main() -> io::Result<()> {   

    match entry() {
        Ok(()) => ExitCode::SUCCESS, 
        Err(_) => ExitCode::FAILURE 
    };
    Ok(())
}

// TODO: Synonym terms
// TODO: Add levenstein distance or cosine similarity
