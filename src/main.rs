use std:: {
    env::{self}, fs::{self, File}, io::{self, BufReader, BufWriter, Read}, path::Path, process::{exit, ExitCode}, str::{self}
};

use xml::{self, reader::XmlEvent, EventReader};
use xml::common::{TextPosition, Position};
use colored::{Colorize};

mod lexer;
mod server;
mod model;

use crate::model::{InMemoryModel, Model, SqliteModel};

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

fn parse_txt_file(file_path: &Path) -> Result<String, ()> {
    let mut content = String::new();
    let mut file = File::open(file_path).map_err(|err| {
        eprintln!("{}: Could not open file {path} as {err}", "ERROR".bold().red(), path = file_path.to_string_lossy().bright_blue(), err = err.to_string().red());
    })?;

    let _ = file.read_to_string(&mut content);
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

        _ => {
            eprintln!("{}: Extension {ext} is unsupported", "ERROR".bold().red(), ext = ext.unwrap());   
            Err(())
        }
    }
}

/* Save the model to a path as json file */
fn save_model_as_json(model: &InMemoryModel, index_path: &str) -> Result<(), ()> {
    println!("Saving folder index to {} ...", index_path);
    
    let index_file = File::create(index_path).map_err(|err| {
        eprintln!("{err}: Failed to create file {path} as {msg}", err = "ERROR".bold().red(), path = index_path.bright_blue(), msg = err.to_string().red());
    })?;

    serde_json::to_writer(BufWriter::new(index_file), &model).unwrap_or_else(|err| {
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
        let file_ext = file_path.extension();

        // Skip unsupported files
        if let Some(ext) = file_ext {
            const ALLOWED_EXTS: [&str; 4] = ["xml", "xhtml", "txt", "md"];
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
            append_folder_to_model(&file_path, model)?;
            continue 'step;
        }   

        println!("Indexing {} ...", file_path_str.bright_cyan());

        let content = match parse_file_by_ext(&file_path) {
            Ok(content) => content.chars().collect::<Vec<_>>(),
            Err(err) => {
                eprintln!("{error}: Failed to read xml file {fp}: {msg:?}", error = "ERROR".bold().red(), fp = file_path.to_str().unwrap().bright_blue(), msg = err);
                continue 'step;
            }
        };
        
        // Core Operation
        model.add_document(file_path, &content)?;
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

const DEFAULT_INDEX_JSON_PATH: &str = "index.json";
const DEFAULT_INDEX_SQLITE_DB_PATH: &str = "index.db";


fn fetch_model(index_path: &str) -> Result<InMemoryModel, ()> {
    let index_file = fs::File::open(&index_path).map_err(|err| {
        eprintln!("{}: Could not open file {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = index_path.bright_blue(), err = err.to_string().red());
    }).unwrap();

    let model: InMemoryModel = serde_json::from_reader(BufReader::new(index_file)).map_err(|err| {
        eprintln!("{}: Serde failed to read {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = index_path.bright_blue(), err = err.to_string().red());
    }).unwrap();

    return Ok(model);
}
    
fn usage(program: &String) {
    eprintln!("{}: {program} [SUBCOMMAND] [OPTIONS]", "USAGE".bold().cyan(), program = program.bright_blue());
    eprintln!("Subcommands:");
    eprintln!("    index  <folder> [save-path]        Index the <folder> containing XML/XHTML files and save the index to [save-path] (Default: index.json)");
    eprintln!("    search <index-file> <prompt>       Search query within a index file. (Default: Shows top 20 search results)");
    eprintln!("    check  [index-file]                Quickly check how many documents are present in a saved index file (Default: index.json)");
    eprintln!("    serve  <index-file> [address]      Starts an HTTP server with Web Interface based on a pre-built index (Default: localhost:6969)");
}

fn entry() -> Result<(), ()> {
    let mut args = env::args();
    let program = args.next().expect("path to program is provided");

    let mut subcommand = None;
    let mut use_sqlite_mode = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sqlite" => use_sqlite_mode = true,
            _ => {
                subcommand = Some(arg);
                break
            }
        }
    }

    let subcommand = subcommand.ok_or_else(|| {
        usage(&program);
        eprintln!("{}: no subcommand is provided", "ERROR".red().bold());
    })?;

    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("{}: No directory path is provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
            })?;

            if use_sqlite_mode {
                let index_path = "index.db";

                // Remove previous index.db to update 
                if Path::exists(Path::new(index_path)) {
                    if let Err(err) = fs::remove_file(index_path) {
                        eprintln!("{}: Could not delete file {path} as \"{err}\"", "ERROR".bold().red(), path = index_path.bold().bright_blue(), err = err.to_string().red());
                        return Err(());
                    }
                }

                let mut model = SqliteModel::open(Path::new(index_path)).unwrap();
                model.begin()?;
                append_folder_to_model(Path::new(&dir_path), &mut model)?;
                model.commit()?;

            } else {
                let index_path = "index.json";
                let mut model = InMemoryModel::default();
                append_folder_to_model(Path::new(&dir_path), &mut model)?;
                save_model_as_json(&model, index_path)?;
            }
        }

        "search" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("{}: Index file path must provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
            })?;

            let prompt = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("{}: Prompt must be provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
            })?.chars().collect::<Vec<char>>();


            if use_sqlite_mode {
                let model = SqliteModel::open(Path::new(&index_path))?;
                for (path, rank) in model.search_query(&prompt)?.iter().take(20) {
                    println!("{path} - {rank}", path = path.display());
                } 
            } else {
                let model = fetch_model(&index_path)?;
                for (path, rank) in model.search_query(&prompt)?.iter().take(20) {
                    println!("{path} - {rank}", path = path.display());
                } 
            }

            return Ok(());
        }

        "check" => {
            if use_sqlite_mode {
                let index_path = args.next().unwrap_or(DEFAULT_INDEX_SQLITE_DB_PATH.to_string());
                let model = SqliteModel::open(Path::new(&index_path))?;
                println!("{info}: Database has {count} entries.", info = "INFO".cyan(), count = model.check().unwrap());
            } else {
                let index_path = args.next().unwrap_or(DEFAULT_INDEX_JSON_PATH.to_string());
                check_index(&index_path).unwrap();
            }
        }

        "serve" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("{}: Index file path must provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
            })?;
            
            // Default address 
            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            
            if use_sqlite_mode {
                let model = SqliteModel::open(Path::new(&index_path))?;
                return server::start(&address, &model);
                
            } else {
                let model: InMemoryModel = fetch_model(&index_path).unwrap_or_else(|()| {
                    eprintln!("{}: Failed to fetch model for {}.", "ERROR".bold().red(), index_path.bright_blue());
                    exit(1);
                });
                return server::start(&address, &model);
            }
        }   

        _ => {

            usage(&program);
            eprintln!("{}: Unknown subcommand {}", "ERROR".bold().red(), subcommand.bold().bright_blue());
            return Err(());
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
