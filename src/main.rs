use std::fs::{self};
use std::io::{self};
use std::path::{Path, PathBuf};
use xml::{self, reader::XmlEvent};
use std::collections::HashMap;
use colored::Colorize;

/**
 *  Parse all the text (Character Events) from the XML File 
 */ 
fn read_xml_file(fp: &str) -> io::Result<String> {
    let file = fs::File::open(fp)?;
    let er = xml::EventReader::new(file);
    let mut content = String::new();
    for event in er.into_iter() {
        if let Ok(XmlEvent::Characters(text)) = event {
            content.push_str(&text);
            content.push_str(" ");  // To differentiate between string token insert additional spaces
        }
    }
    return Ok(content);
}

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

fn index_document(fp: &str) -> io::Result<HashMap<String, usize>> {
    /* Returns frequency table of a document containing mapping of term along with its frequency of occurence */
    let content = read_xml_file(&fp)?.chars().collect::<Vec<_>>();
    let lexer = Lexer::new(&content);


    let mut ft = HashMap::<String, usize>::new();
    for token in lexer {
        let token = token.iter().collect::<String>().to_uppercase();
        ft.entry(token).and_modify(|x| *x += 1).or_insert(1);
    }

    Ok(ft)
}

// Associative types
type FreqTable = HashMap::<String, usize>;
type FreqTableIndex = HashMap<PathBuf, FreqTable>;

const INDEX_FILE_PATH: &str = "index.json";

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

fn main() -> io::Result<()> {
    
    let dir = fs::read_dir("docs.gl/gl4")?;

    let index_fp_path_buf = Path::new(INDEX_FILE_PATH).to_path_buf();
    let mut tf_index = fetch_index(index_fp_path_buf)?;

    if tf_index.is_empty() {
        /* Incase not possible to fetch index start over indexing again */ 
        for (i, file) in dir.into_iter().enumerate() {
            let file_path = file?.path();
            // let file_path = file_path.to_str().unwrap();
            let freq_table: FreqTable = index_document(file_path.to_str().unwrap())?;

            let file_path_string = file_path.to_str().unwrap().to_string();
            println!("{i} Indexing {file_path} ...", i = i.to_string().italic(), 
                                                    file_path = file_path_string.bold().blue());
            tf_index.insert(file_path, freq_table);    
        }
    }

    print_statistics(&tf_index);
    println!();

    // let index_fp = Path::new("index.json");    
    // update_index(index_fp.to_path_buf(), &tf_index)?;

    // for (path, freq_table) in tf_index {
    //     println!("{path:3>?} has {count} unique terms in it", count = freq_table.len().to_string().bold().green());
    // }

    return Ok(());
}
