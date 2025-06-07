use std::fs::{self};
use std::io::{self};
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

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left();

        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            let mut n = 0;
            while n < self.content.len() && self.content[n].is_numeric() {
                n += 1;
            }

            let token = &self.content[0..n]; 
            self.content = &self.content[n..];
            return Some(token);
        }

        if self.content[0].is_alphabetic() {
            let mut n = 0;
            while n < self.content.len() && self.content[n].is_alphanumeric() {
                n += 1;
            }

            let token = &self.content[0..n]; 
            self.content = &self.content[n..];
            return Some(token);
        }

        let token = &self.content[0..1]; 
        self.content = &self.content[1..];
        // eprintln!("{}:{}", "Invalid token encountered ".bold().red(), token.iter().collect::<String>().bold().white());
        return Some(token);
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];
    fn next(&mut self) -> Option<<Self as Iterator>::Item> { 
        self.next_token()
    }
}

#[allow(dead_code)]
fn index_document(fp: &str) -> HashMap<String, usize> {
    /* Get frequency table for each document */
    // let ft = HashMap::<String, usize>::new();
    // let content = read_xml_file(fp);
    // return ft;
    unimplemented!();
}

fn main() -> io::Result<()> {
    // Main Document Frequency table
    let dft = HashMap::<String, HashMap<String, usize>>::new();

    let fp = "docs.gl/gl4/glAttachShader.xhtml";

    let content = read_xml_file(&fp)?.chars().collect::<Vec<_>>();
    let lexer = Lexer::new(&content);

    for token in lexer {
        println!("{:?}", token.iter().map(|x| x.to_ascii_uppercase()).collect::<String>()); 
    }

    // println!("File {fp}: \n\n{content}", fp = fp.to_uppercase());

    // let dir = fs::read_dir("../docs.gl/gl4")?;

    // for file in dir {
    //     let file_path = file?.path();
    //     let content= parse_xml_file(file_path.to_str().unwrap());
    //     println!("{:?} => {}", file_path, content?.len());
    // }

    return Ok(());
}
