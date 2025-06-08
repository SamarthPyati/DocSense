use std::fs::{self, File};
use std::io::{self};
use std::env::{self};
use std::process::{exit};
use std::path::{PathBuf, Path};
use xml::{self, reader::XmlEvent, EventReader};
use xml::common::{TextPosition, Position};
use std::collections::HashMap;
use colored::Colorize;

#[derive(Debug)]
struct Lexer<'a> {
    // Lifetimes implemented as content not owned
    // As used it will be assigned or shifted
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
        /* Return a slice of n len on predicate being true */
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        return self.chop(n);
    }

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left();

        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            return Some(self.chop_while(|x| x.is_numeric()));
        }

        if self.content[0].is_alphabetic() {
            return Some(self.chop_while(|x| x.is_alphanumeric()));
        }

        let token = self.chop(1);
        // eprintln!("{}:{}", "Invalid token encountered ".bold().red(), 
                            // token.iter().collect::<String>().bold().white());
        return Some(token);
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];
    fn next(&mut self) -> Option<<Self as Iterator>::Item> { 
        self.next_token()
    }
}

/* Parse all the text (Character Events) from the XML File */
fn read_xml_file(file_path: &Path) -> Result<String, ()> {
    let file = File::open(file_path).map_err(|err| {
        eprintln!("ERROR: could not open file {file_path}: {err}", file_path = file_path.display());
    })?;
    let er = EventReader::new(file);
    let mut content = String::new();
    for event in er.into_iter() {
        let event = event.map_err(|err| {
            let TextPosition {row, column} = err.position();
            let msg = err.msg();
            eprintln!("{file_path}:{row}:{column}: {err}: {msg}", err = "ERROR".red().bold(), file_path = file_path.display());
        })?;

        if let XmlEvent::Characters(text) = event {
            content.push_str(&text);
            content.push(' ');
        }
    }
    Ok(content)
}

/* Returns frequency table of a document containing mapping of term along with its frequency of occurence */
fn index_document(fp: &Path) -> io::Result<HashMap<String, usize>> {
    let content = match read_xml_file(fp) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("{err}: Failed to read xml file {fp:?}: {msg:?}", err = "ERROR".bold().red(), fp = fp, msg = error);
            return Ok(HashMap::new()); // Return empty map to continue
        }
    };

    let content = content.chars().collect::<Vec<_>>();
    let lexer = Lexer::new(&content);


    let mut ft = HashMap::<String, usize>::new();
    for token in lexer {
        let token = token.iter().collect::<String>().to_uppercase();
        ft.entry(token).and_modify(|x| *x += 1).or_insert(1);
    }

    Ok(ft)
}

/* Indexes a particular folder to 'index.json' */
fn index_folder(dir_path: &str, index_path: Option<&'static str>) -> io::Result<()> {
    let dir = fs::read_dir(dir_path)?;
    let mut tf_index: FreqTableIndex = FreqTableIndex::new();

    for file in dir {
        let file_path = file?.path();
        println!("Indexing {:?} ...", file_path);

        let tf: FreqTable = index_document(file_path.as_path())?;
        tf_index.insert(file_path, tf);
    }

    let index_path = index_path.unwrap_or(DEFAULT_INDEX_FILE_PATH);
    println!("Saving folder index to {} ...", index_path);
    let index_file = File::create(index_path)?;
    serde_json::to_writer(index_file, &tf_index)?;
    Ok(())
}

/* Check the amount of files present in the main frequency table index */
fn check_index(index_fp: &str) -> io::Result<()> {
    let index_file = fs::File::open(index_fp).unwrap_or_else(|err| {
        eprintln!("{}: could not open file {file} as \"{err}\"", "ERROR".bold().red(), file = index_fp.to_string().bold(), err = err.to_string().red());
        exit(1);
    });
    println!("Reading {} file ...", index_fp);
    let tf_index: FreqTableIndex = serde_json::from_reader(index_file).unwrap_or_else(|err|  {
        eprintln!("{}: serde could not read file {file} as \"{err}\"", "ERROR".bold().red(), file = index_fp.to_string().bold(), err = err.to_string().red());
        exit(1);
    });
    println!("Index file has {} entries.", tf_index.len());
    Ok(())  
}

// Associative types
type FreqTable = HashMap::<String, usize>;
type FreqTableIndex = HashMap<PathBuf, FreqTable>;

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

fn usage(program: &String) {
    eprintln!("{}: {program} [SUBCOMMAND] [OPTIONS]", "USAGE".bold().cyan(), program = program.bright_blue());
    eprintln!("Subcommands:");
    eprintln!("    index <folder>         index the <folder> and save the index to index.json file");
    eprintln!("    check <index-file>     check how many documents are indexed in the file (searching is not implemented yet)");
}

fn main() -> io::Result<()> {   

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
                eprintln!("{}: no directory path is provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
                exit(1);
            });

            index_folder(&dir_path, Some(DEFAULT_INDEX_FILE_PATH)).unwrap_or_else(|err| {
                eprintln!("{}: failed to index folder {} as \"{}\"", "ERROR".bold().red(), dir_path.bold(), err.to_string().red());
                exit(1);
            });
        }

        "check" => {
            let index_path = args.next().unwrap_or_else(|| {
                println!("{}: no index path is provided for {} subcommand.", "ERROR".bold().red(), subcommand.bold().bright_blue());
                exit(1);
            });

            check_index(&index_path).unwrap_or_else(|err| {
                println!("{}: could not check index file {index_path} as \"{err}\"", "ERROR".bold().red());
                exit(1);
            });
        }

        _ => {

            usage(&program);
            eprintln!("{}: Unknown subcommand {}", "ERROR".bold().red(), subcommand.bold().bright_blue());
            exit(1);
        }
    }

    // let dir = fs::read_dir("docs.gl/gl4")?;

    // let index_fp_path_buf = Path::new(DEFAULT_INDEX_FILE_PATH).to_path_buf();
    // let mut tf_index = fetch_index(index_fp_path_buf)?;

    // check_index("index.json")?;

    // if tf_index.is_empty() {
    //     /* Incase not possible to fetch index start over indexing again */ 
    //     for (i, file) in dir.into_iter().enumerate() {
    //         let file_path = file?.path();
    //         // let file_path = file_path.to_str().unwrap();
    //         let freq_table: FreqTable = index_document(file_path.to_str().unwrap())?;

    //         let file_path_string = file_path.to_str().unwrap().to_string();
    //         println!("{i} Indexing {file_path} ...", i = i.to_string().italic(), 
    //                                                 file_path = file_path_string.bold().blue());
    //         tf_index.insert(file_path, freq_table);    
    //     }
    // }

    // // print_statistics(&tf_index);
    // println!();

    return Ok(());
}
