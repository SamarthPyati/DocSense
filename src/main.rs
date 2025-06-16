use std:: {
    fs::{self, File}, 
    io::{self}, 
    env::{self}, 
    process::{exit, ExitCode}, 
    path::{PathBuf, Path}, 
    collections::HashMap, 
    str::{self},
};

use xml::{self, reader::XmlEvent, EventReader};
use xml::common::{TextPosition, Position};

use tiny_http::{Method, Request, Response, Server, StatusCode};
use colored::Colorize;

#[derive(Debug)]
struct Lexer<'a> {
    // Lifetimes implemented as content is not owned
    // as used it will be assigned or shifted
    content: &'a [char]
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left(&mut self) {
        // Get rid of trailing whitespace
        while self.content.len() > 0 && self.content[0].is_whitespace() {
            // Skip the current char and assign to next
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        /* Return a slice of n len */
        let token = &self.content[0..n]; 
        self.content = &self.content[n..];
        token
    }

    fn chop_while<P>(&mut self, mut predicate: P) -> &'a [char]
    where
        P: FnMut(&char) -> bool,
    {
        /* Return a chopped slice of n length on predicate being true */
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        return self.chop(n);
    }

    fn next_token(&mut self) -> Option<String> {
        self.trim_left();

        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            // Ignore single digit number 
            let result = self.chop_while(|x| x.is_numeric());
            if result.len() == 1 { return None; }
            return Some(result.iter().collect());
        }

        if self.content[0].is_alphabetic() {
            let result = self.chop_while(|x| x.is_alphanumeric());
            return Some(result.iter().map(|x| x.to_ascii_uppercase()).collect());
        }
        
        let unwanted_symbols  = &[',', ';', '*', '/', '?', '{', '}', '(', ')', '.', '$', '_', '-'];
        
        // Ignore single-character unwanted punctuation
        if unwanted_symbols.contains(&self.content[0]) {
            self.chop(1);   // skip this token 
            return self.next_token();     // recursively fetch next token 
        }

        let token = self.chop(1);
        return Some(token.iter().collect());
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> { 
        self.next_token()
    }
}

/* Parse all the text (Character Events) from the XML File */
fn read_xml_file(file_path: &Path) -> Result<String, ()> {
    let file = File::open(file_path).map_err(|err| {
        eprintln!("{}: Could not open file {file_path}: {err}", "ERROR".bold().red(), file_path = file_path.display());
    })?;
    let er = EventReader::new(file);
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

/* Returns frequency table of a document containing mapping of term along with its frequency of occurence */
fn index_document(fp: &Path) -> io::Result<FreqTable> {
    let content = match read_xml_file(fp) {
        Ok(content) => content,
        Err(error) => {
            let fp_str= fp.to_str().unwrap();
            eprintln!("{err}: Failed to read xml file {fp}: {msg:?}", err = "ERROR".bold().red(), fp = fp_str.bright_blue(), msg = error);
            return Ok(HashMap::new()); // Return empty map to continue
        }
    };

    let content = content.chars().collect::<Vec<_>>();
    let lexer = Lexer::new(&content);

    let mut ft = HashMap::<String, usize>::new();
    for token in lexer {
        ft.entry(token).and_modify(|x| *x += 1).or_insert(1);
    }

    Ok(ft)
}

/* Save the Frequency Table Index to a path as index.json */
fn save_index(tf_index: &FreqTableIndex, index_path: &str) -> io::Result<()> {
    println!("Saving folder index to {} ...", index_path);
    let index_file = File::create(index_path)?;
    serde_json::to_writer(index_file, tf_index).unwrap_or_else(|err| {
        eprintln!("{err}: Failed to save {fp:?}: {msg:?}", err = "ERROR".bold().red(), fp = index_path, msg = err);
    });
    Ok(())
}

/* Indexes a particular folder to 'index.json' */
fn index_folder(dir_path: &str, tf_index: &mut FreqTableIndex) -> Result<(), ()> {
    let dir = fs::read_dir(dir_path).map_err(|err| {
        eprintln!("{err}: Failed to read directory {dir_path} as \"{msg}\"", err = "ERROR".bold().red(), 
                                                                            dir_path = dir_path.bold().bright_blue(), 
                                                                            msg = err.to_string().red());
    })?;

    'step: for file in dir {
        let file = file.map_err(|err| {
            eprintln!("{err}: Failed to read next file in directory {dir_path} as {msg}", err = "ERROR".bold().red(), dir_path = dir_path.bold().bright_blue(), msg = err.to_string().red());
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

        if file_type.is_dir() {
            // Recursively index all the folders
            let _ = index_folder(file_path_str, tf_index);
            continue 'step;
        }   

        println!("Indexing {} ...", file_path_str.bright_cyan());

        let tf: FreqTable = index_document(file_path.as_path()).map_err(|err| {
            eprintln!("{err}: Failed to index document {file_path} as {msg}", err = "ERROR".bold().red(), file_path = file_path_str.bold().bright_blue(), msg = err.to_string().red());
        })?;
        tf_index.insert(file_path, tf);
    }
    Ok(())
}

/* Check the amount of files present in the main frequency table index */
fn check_index(index_fp: &str) -> io::Result<()> {
    let index_file = fs::File::open(index_fp).unwrap_or_else(|err| {
        eprintln!("{}: Could not open file {file} as \"{err}\"", "ERROR".bold().red(), file = index_fp.to_string().bright_blue(), err = err.to_string().red());
        exit(1);
    });
    println!("{info}: Reading file {file}", info = "INFO".bright_cyan(), file = index_fp);
    let tf_index: FreqTableIndex = serde_json::from_reader(index_file).unwrap_or_else(|err|  {
        eprintln!("{}: Serde could not read file {file} as \"{err}\"", "ERROR".bold().red(), file = index_fp.to_string().bold(), err = err.to_string().red());
        exit(1);
    });
    println!("{info}: Index file has {entries} entries", info = "INFO".bright_cyan(), entries = tf_index.len());
    Ok(())  
}

// Associative types
type FreqTable = HashMap::<String, usize>;
type FreqTableIndex = HashMap::<PathBuf, FreqTable>;

const DEFAULT_INDEX_FILE_PATH: &str = "index.json";

#[allow(dead_code)]
fn fetch_index(index_fp: PathBuf) -> io::Result<FreqTableIndex> {
    let metadata = fs::metadata(&index_fp)?;
    if metadata.len() == 0 {
        return Ok(FreqTableIndex::new()); // return empty index
    }

    let index_file = fs::File::open(index_fp)?;
    let index: FreqTableIndex = serde_json::from_reader(index_file)?;
    Ok(index)
}

#[allow(dead_code)]
fn update_index(index_fp: PathBuf, tf_index: &FreqTableIndex) -> io::Result<()>{
    let metadata = fs::metadata(&index_fp)?;
    if metadata.len() == 0 {
        println!("NOTE: Empty {:?} file, writing to it ...", index_fp);
        let index_file = fs::File::create(index_fp)?;
        serde_json::to_writer(index_file, &tf_index).expect("Saved index.json file to disk.");
    }
    Ok(())
}

#[allow(dead_code)]
fn print_statistics(index: &FreqTableIndex) {
    for (i, (file_path, freq_table)) in index.iter().enumerate() {

        /* Printing Statistics */
        let mut stats = freq_table.iter().collect::<Vec<_>>();
        stats.sort_by_key(|(_, freq)| *freq);
        stats.reverse();
        
        let file_path_string = file_path.to_str().unwrap().to_string();
        /* Print Top 10 most frequently occuring terms */
        println!("{} #{}: {file_path:} ", 
                                "FILE".italic().underline(), 
                                (i + 1).to_string().bold().italic(), 
                                file_path = file_path_string.bold().bright_blue());
    
        for (term, freq) in stats.iter().take(20) {
            println!("   {term} : {freq}");
        }    
    }
}

fn serve_static_file(request: Request, file_path: &str) -> Result<(), ()> {
    let html_file = File::open(Path::new(file_path)).map_err(|err| {
        eprintln!("{}: Could not open html file {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = file_path.bright_blue(), err = err.to_string().red());
    }).unwrap();
    
    let res = Response::from_file(html_file);
    return request.respond(res).map_err(|err| {
        eprintln!("{}: Could not serve request for {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = file_path.bright_cyan(), err = err.to_string().red());
    });
}

fn serve_404(request: Request) -> Result<(), ()> {
    return request.respond(Response::from_string("404")
            .with_status_code(StatusCode(404)))
            .map_err(|err| {
                eprintln!("{}: Could not serve request as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
            });
}

fn serve_request(mut request: Request) -> Result<(), ()> {
    println!("{info}: Received request! method: [{req}], url: {url:?}",
        info = "INFO".bright_cyan(), 
        req = &request.method().as_str().bright_green(),
        url = &request.url()
    );

    match (&request.method(), request.url()) {
        
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "src/index.html")?;
        }

        (Method::Get, "/index.js") => {
            serve_static_file(request, "src/index.js")?;
        }

        (Method::Post, "/api/search") => {
            let mut buf = Vec::new();
            request.as_reader().read_to_end(&mut buf).map_err(|err| {
                eprintln!("{}: Could not read body of request as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
            })?;

            let body = str::from_utf8(&mut buf).map_err(|err| {
                eprintln!("{}: Could not interpret body as UTF-8 string as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
            })?.chars().collect::<Vec<_>>();

            for token in Lexer::new(&body) {
                println!("{:?}", token);
            }

            request.respond(Response::from_string("Ok")).map_err(|err| {
                eprintln!("{}: \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
            })?;
        }

        _ => {
            serve_404(request)?
        }
    }

    Ok(())
}

fn usage(program: &String) {
    eprintln!("{}: {program} [SUBCOMMAND] [OPTIONS]", "USAGE".bold().cyan(), program = program.bright_blue());
    eprintln!("Subcommands:");
    eprintln!("    index <folder> <save-path>         Index the <folder> and save the index to <save-path> (Default: index.json)");
    eprintln!("    check [index-file]                 Check how many documents are indexed in the file (Default: index.json)");
    eprintln!("    serve [address]                    Opens a HTTP Server to specified address for getting query (Default: localhost:6969)");
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

            let mut tf_index: FreqTableIndex = FreqTableIndex::new();

            let _ = index_folder(&dir_path,&mut tf_index).map_err(|err| {
                eprintln!("{}: Failed to index folder {dir_path} as \"{err:?}\"", "ERROR".bold().red(), dir_path = dir_path.bold().bright_blue(), err = err);
            });

            let save_path = args.next().unwrap_or("index.json".to_string());
            let save_path= save_path.as_str();
            // Default path is set to 'index.json'
            save_index(&tf_index, save_path)?;
        }

        "check" => {
            // May be remove the default 'index.json' and ask user to provide path 
            // let index_path = args.next().unwrap_or_else(|| {
            //     println!("{}: No index path is provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
            //     exit(1);
            // });
            let index_path = args.next().unwrap_or(DEFAULT_INDEX_FILE_PATH.to_string());

            check_index(&index_path).unwrap_or_else(|err| {
                println!("{}: Could not check index file {index_path} as \"{err}\"", "ERROR".bold().red());
                exit(1);
            });
        }

        "serve" => {
            let address = args.next().unwrap_or("127.0.0.1:6969".to_string());
            let address_str = "http://".to_string() + &address + "/";   // Weird ass rust string concat
            println!("{info}: Server Listening at: {address}", info = "INFO".bright_cyan(), address = address_str.cyan());
            let server = Server::http(address).unwrap();

            for request in server.incoming_requests() {
                let _ = serve_request(request);
            }
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
