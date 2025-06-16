use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use std::{
    self, fs::File, io::{self, ErrorKind}, path::{Path, PathBuf}, process::exit, str
}; 

use std::collections::HashMap;
use colored::Colorize;

use super::lexer::*;


// Associative types
type FreqTable = HashMap::<String, usize>;
type FreqTableIndex = HashMap::<PathBuf, FreqTable>;


fn tf(term: &str, freq_table: &FreqTable) -> f32 {
    let n = *freq_table.get(term).unwrap_or(&0) as f32;
    // NOTE: Can lead to division by 0 if term is not in FreqTable
    // Workaround:  -> (So add 1 to denominator to prevent that (Getting negative values => REJECTED))
    //              -> Take either max of denom or 1 => APPROVED
    let d = freq_table.iter().map(|(_, c)| *c).sum::<usize>().max(1) as f32;   
    n / d
}


fn idf(term: &str, index: &FreqTableIndex) -> f32 {
    let n = index.len() as f32;
    // NOTE: Can lead to division by 0 if term is not in Document Corpus
    let d  = index.values().filter(|ft| ft.contains_key(term)).count().max(1) as f32;
    f32::log10(n / d)
}


pub fn serve_404(request: Request) -> io::Result<()> {
    return request.respond(Response::from_string("404").with_status_code(StatusCode(404)));
}


pub fn serve_500(request: Request) -> io::Result<()> {
    return request.respond(Response::from_string("404").with_status_code(StatusCode(500)));
}


pub fn serve_400(request: Request, message: &str) -> io::Result<()> {
    return request.respond(Response::from_string(format!("400: {message}")).with_status_code(StatusCode(500)));
}


pub fn serve_static_file(request: Request, file_path: &str) -> io::Result<()> {
    let html_file = match File::open(Path::new(file_path)) {
        Ok(file) => file, 
        Err(err) => {
            eprintln!("{}: Could not open html file {file_path} as \"{err}\"", "ERROR".bold().red(), file_path = file_path.bright_blue(), err = err.to_string().red());
            if err.kind() == ErrorKind::NotFound {
                return serve_404(request);
            }
            return serve_500(request);
        }
    };
    
    return request.respond(Response::from_file(html_file));
}


pub fn serve_api_search(mut request: Request, tf_index: &FreqTableIndex) -> io::Result<()>{
    let mut buf = Vec::new();
    // Read the entire body of request 
    if let Err(err) = request.as_reader().read_to_end(&mut buf) {
        eprintln!("{}: Could not read body of request as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
        return serve_500(request);
    }

    let body = match str::from_utf8(&mut buf) {
        Ok(body) => body.chars().collect::<Vec<_>>(), 
        Err(err) => {
            eprintln!("{}: Could not interpret body as UTF-8 string as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
            return serve_400(request, "Body must be a valid UTF-8 string");
        }
    };

    println!("Recieved Query: \'{}\'", body.iter().collect::<String>().bright_blue());

    let results = search_query(&body, tf_index);
    
    // Display document ranks
    for (path, rank) in results.iter().take(10) {
        println!("      {} => {}", path.display(), rank);
    }

    let content= &results.iter().take(20).collect::<Vec<_>>();
    let json = match serde_json::to_string(content) {
        Ok(json) => json, 
        Err(err) => {
            eprintln!("{}: could not convert search results to JSON as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
            return serve_500(request);
        }
    };

    let content_header = Header::from_bytes("Content-Type", "application/json")
                                                    .expect("Header entered is not a garbage value");
    
    let response = Response::from_string(json).with_header(content_header);

    return request.respond(response);
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


pub fn serve_request(tf_index: &FreqTableIndex, request: Request) -> io::Result<()> {
    println!("{info}: Received request! method: [{req}], url: {url:?}",
        info = "INFO".bright_cyan(), 
        req = &request.method().as_str().bright_green(),
        url = &request.url()
    );

    match (&request.method(), request.url()) {
        
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "src/index.html")?
        }

        (Method::Get, "/index.js") => {
            serve_static_file(request, "src/index.js")?
        }

        (Method::Post, "/api/search") => {
            serve_api_search(request, tf_index)?
        }

        _ => {
            return serve_404(request);
        }
    }

    Ok(())
}


pub fn server_start(address: &str, tf_index: &FreqTableIndex) -> io::Result<()> {
    let address_str = "http://".to_string() + &address + "/"; 
    let server = Server::http(address).map_err(|err| {
        eprintln!("{}: Could not create initiate server at {address} as \"{err}\"", "ERROR".bold().red(), address = address.bold().bright_blue(), err = err.to_string().red());
        exit(1);
    }).unwrap();

    println!("{info}: Server Listening at: {address}", info = "INFO".bright_cyan(), address = address_str.cyan());

    for request in server.incoming_requests() {
        serve_request(&tf_index, request).map_err(|err| {
            eprintln!("{}: Failed to serve the request as \"{err}\"", "ERROR".bold().red(), err = err.to_string().red());
        }).ok(); // <- Don't stop here continue serving requests
    }
    eprintln!("{}: Server socket has shutdown", "ERROR".bold().red());
    Ok(())
}
