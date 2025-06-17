use std:: {
    collections::HashMap, env::{self}, fs::{self, File}, io::{self, BufReader, BufWriter}, path::{Path, PathBuf}, process::{exit, ExitCode}, str::{self}
};

use xml::{self, reader::XmlEvent, EventReader};
use xml::common::{TextPosition, Position};
use colored::Colorize;

mod lexer;
mod server;
mod model;

use crate::model::{InMemoryModel, Model, SqliteModel};

/* Parse all the text (Character Events) from the XML File */
fn read_xml_file(file_path: &Path) -> Result<String, ()> {
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


/* Save the model to a path as json file */
#[allow(dead_code)]
fn save_model(model: &InMemoryModel, index_path: &str) -> Result<(), ()> {
    println!("Saving folder index to {} ...", index_path);
    
    let index_file = File::create(index_path).map_err(|err| {
        eprintln!("{err}: Failed to create file {path} as {msg}", err = "ERROR".bold().red(), path = index_path.bright_blue(), msg = err.to_string().red());
    })?;

    serde_json::to_writer(BufWriter::new(index_file), model).unwrap_or_else(|err| {
        eprintln!("{err}: Failed to save file {path} as {msg}", err = "ERROR".bold().red(), path = index_path.bright_blue(), msg = err.to_string().red());
    });
    Ok(())
}

/* Indexes a folder as a json file and adds to model */
fn append_folder_to_model(dir_path: &Path, model: &mut dyn Model) -> Result<(), ()> {
    let dir = fs::read_dir(dir_path).map_err(|err| {
        eprintln!("{err}: Failed to read directory {dir_path} as \"{msg}\"", err = "ERROR".bold().red(), 
                                                                            dir_path = dir_path.to_str().unwrap().bold().bright_blue(), 
                                                                            msg = err.to_string().red());
    })?;

    'step: for file in dir {
        let file = file.map_err(|err| {
            eprintln!("{err}: Failed to read next file in directory {dir_path} as {msg}", err = "ERROR".bold().red(), 
                                                                                        dir_path = dir_path.to_str().unwrap().bold().bright_blue(), 
                                                                                        msg = err.to_string().red());
        })?;

        let file_path = file.path();
        let file_path_str = file_path.to_str().unwrap();

        // Skip unsupported files
        if let Some(ext) = file_path.extension() {
            const ALLOWED_EXTS: [&str; 2] = ["xml", "xhtml"];
            if !ALLOWED_EXTS.contains(&ext.to_str().unwrap()) {
                println!("{}: Skipping non-XML file {}", "INFO".cyan(), file_path_str.bright_yellow());
                continue 'step;
            }
        }

        // If file is another directory recursively index it too 
        let file_type = file.file_type().map_err(|err| {
            eprintln!("{err}: Failed to determine file type for {file_path} as {msg}", err = "ERROR".bold().red(), file_path = file_path_str.bold().bright_blue(), msg = err.to_string().red());
        })?;

        // Recursively index all the folders
        if file_type.is_dir() {
            let _ = append_folder_to_model(&file_path, model);
            continue 'step;
        }   

        println!("Indexing {} ...", file_path_str.bright_cyan());

        let content = match read_xml_file(&file_path) {
            Ok(content) => content.chars().collect::<Vec<_>>(),
            Err(err) => {
                eprintln!("{error}: Failed to read xml file {fp}: {msg:?}", error = "ERROR".bold().red(), fp = file_path.to_str().unwrap().bright_blue(), msg = err);
                continue 'step;
            }
        };

        let _ = model.add_document(file_path, &content);
    }
    Ok(())
}

/* Check the amount of files present in the main frequency table index */
fn check_index(index_path: &str) -> Result<(), ()> {
    let index_file = fs::File::open(index_path).map_err(|err| {
        eprintln!("{}: Could not open file {file} as \"{err}\"", "ERROR".bold().red(), file = index_path.bright_blue(), err = err.to_string().red());
        exit(1);
    })?;

    println!("{info}: Reading file {file}", info = "INFO".bright_cyan(), file = index_path.bright_blue());
    let model: InMemoryModel = serde_json::from_reader(BufReader::new(index_file)).map_err(|err|  {
        eprintln!("{}: Serde could not read file {file} as \"{err}\"", "ERROR".bold().red(), file = index_path.bright_blue(), err = err.to_string().red());
        exit(1);
    })?;
    println!("{info}: Index file has {entries} entries", info = "INFO".bright_cyan(), entries = model.tf_index.len());
    Ok(())  
}

const DEFAULT_INDEX_FILE_PATH: &str = "index.json";

fn fetch_model(index_path: &str) -> io::Result<InMemoryModel> {
    let metadata = fs::metadata(&index_path).map_err(|err|{
        eprintln!("{}: Could not fetch metadata of {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = index_path.bright_blue(), err = err.to_string().red());
    }).unwrap();
    
    if metadata.len() == 0 {
        return Ok(InMemoryModel{ gtf: HashMap::new(), tf_index: HashMap::new() }); // return empty index to continue
    }

    let index_file = fs::File::open(&index_path)?;
    let model: InMemoryModel = serde_json::from_reader(BufReader::new(index_file)).map_err(|err| {
        eprintln!("{}: Serde failed to read {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = index_path.bright_blue(), err = err.to_string().red());
    }).unwrap();
    Ok(model)
}

fn usage(program: &String) {
    eprintln!("{}: {program} [SUBCOMMAND] [OPTIONS]", "USAGE".bold().cyan(), program = program.bright_blue());
    eprintln!("Subcommands:");
    eprintln!("    index  <folder> [save-path]        Index the <folder> containing XML/XHTML files and save the index to [save-path] (Default: index.json)");
    eprintln!("    search <index-file> <prompt>       Search query within a index file. (Default: Shows top 20 search results)");
    eprintln!("    check  [index-file]                Quickly check how many documents are present in a saved index file (Default: index.json)");
    eprintln!("    serve  <index-file> [address]      Starts an HTTP server with Web Interface based on a pre-built index (Default: localhost:6969)");
}

fn entry() -> io::Result<()> {
    let mut args = env::args();
    let program = args.next().expect("Path to program must be provided.");

    let subcommand = args.next().unwrap_or_else(|| {
        usage(&program);
        eprintln!("{}: no subcommand is provided.", "ERROR".bold().red());
        exit(1);
    });


    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().unwrap_or_else(|| {
                eprintln!("{}: No directory path is provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
                exit(1);
            });

            let index_path = "index.db";
            let mut model = SqliteModel::open(Path::new(index_path)).unwrap();

            let _ = model.begin();
            let _ = append_folder_to_model(Path::new(&dir_path), &mut model).map_err(|err| {
                eprintln!("{}: Failed to index folder {dir_path} as \"{err:?}\"", "ERROR".bold().red(), dir_path = dir_path.bold().bright_blue(), err = err);
            });
            let _ = model.commit();

            // let save_path = args.next().unwrap_or("index.json".to_string());
            // save_model(&model, save_path.as_str())?;
        }

        "search" => {
            let index_path = args.next().unwrap_or_else(|| {
                eprintln!("{}: Index file path must provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
                exit(1);
            });

            let prompt = args.next().unwrap_or_else(|| {
                eprintln!("{}: Prompt must be provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
                exit(1);
            }).chars().collect::<Vec<char>>();

            let model: InMemoryModel = fetch_model(&index_path).unwrap();

            for (path, rank) in model.search_query(&prompt).unwrap().iter().take(20) {
                println!("{path} - {rank}", path = path.display());
            }

            return Ok(());
        }

        "check" => {
            let index_path = args.next().unwrap_or(DEFAULT_INDEX_FILE_PATH.to_string());

            check_index(&index_path).unwrap();
        }

        "serve" => {
            let index_path = args.next().unwrap_or_else(|| {
                eprintln!("{}: Index file path must provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
                exit(1);
            });

            let model: InMemoryModel = fetch_model(&index_path).unwrap();

            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            return server::start(&address, &model);
        }   

        _ => {

            usage(&program);
            eprintln!("{}: Unknown subcommand {}", "ERROR".bold().red(), subcommand.bold().bright_blue());
            exit(1);
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {   
    
    match entry() {
        Ok(()) => ExitCode::SUCCESS, 
        Err(_) => ExitCode::FAILURE 
    };

    return Ok(());
}
