use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use std::{
    self, cmp::Ordering, fs::File, io::{self, ErrorKind}, path::Path, process::exit, str, sync::{Arc, Mutex}
}; 

use colored::Colorize;
use super::model::*;

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
            eprintln!("{}: Could not open html file {file_path} as {err}", "ERROR".bold().red(), file_path = file_path.bright_blue(), err = err.to_string().red());
            if err.kind() == ErrorKind::NotFound {
                return serve_404(request);
            }
            return serve_500(request);
        }
    };
    
    return request.respond(Response::from_file(html_file));
}


pub fn serve_api_search(mut request: Request, model: Arc<Mutex<InMemoryModel>>) -> io::Result<()>{
    let mut buf = Vec::new();
    // Read the entire body of request 
    if let Err(err) = request.as_reader().read_to_end(&mut buf) {
        eprintln!("{}: Could not read body of request as {err}", "ERROR".bold().red(), err = err.to_string().red());
        return serve_500(request);
    }

    let body = match str::from_utf8(&mut buf) {
        Ok(body) => body.chars().collect::<Vec<_>>(), 
        Err(err) => {
            eprintln!("{}: Could not interpret body as UTF-8 string as {err}", "ERROR".bold().red(), err = err.to_string().red());
            return serve_400(request, "Body must be a valid UTF-8 string");
        }
    };

    println!("Recieved Query: \'{}\'", body.iter().collect::<String>().bright_blue());

    let model = model.lock().unwrap();
    let results = match model.search_query(&body) {
        Ok(results) => results, 
        Err(()) => return serve_500(request)
    };
    
    let mut content= Vec::new();
    // Display document ranks (if rank turns 0 while iterating, stop from there)
    for (path, rank) in results.iter().take(10) {
        if rank.partial_cmp(&0f32) == Some(Ordering::Equal) {
            break;
        }
        println!("      {} => {}", path.display(), rank);
        content.push((path, rank));
    } 

    let json = match serde_json::to_string(&content) {
        Ok(json) => json, 
        Err(err) => {
            eprintln!("{}: could not convert search results to JSON as {err}", "ERROR".bold().red(), err = err.to_string().red());
            return serve_500(request);
        }
    };

    let content_header = Header::from_bytes("Content-Type", "application/json")
                                                    .expect("Header entered is not a garbage value");
    
    let response = Response::from_string(json).with_header(content_header);

    return request.respond(response);
}

pub fn serve_api_stats(request: Request, model: Arc<Mutex<InMemoryModel>>) -> io::Result<()> {
    use serde::Serialize;
    #[derive(Default, Serialize)]
    struct Stats {
        doc_count: usize, 
        unique_term_count: usize, 
    }

    let model = model.lock().unwrap();
    let stats = Stats {
        doc_count: model.docs.len(), 
        unique_term_count: model.gtf.len()
    };

    let json = match serde_json::to_string(&stats) {
        Ok(json) => json, 
        Err(err) => {
            eprintln!("{}: could not convert search results to JSON as {err}", "ERROR".bold().red(), err = err.to_string().red());
            return serve_500(request);
        }
    };

    let content_header = Header::from_bytes("Content-Type", "application/json")
                                                    .expect("Header entered is not a garbage value");
    
    let response = Response::from_string(json).with_header(content_header);

    return request.respond(response);
}

pub fn serve_request(request: Request, model: Arc<Mutex<InMemoryModel>>) -> io::Result<()> {
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
            serve_api_search(request, model)?
        }

        (Method::Get, "/api/stats") => {
            serve_api_stats(request, model)?
        }

        _ => {
            return serve_404(request);
        }
    }

    Ok(())
}


pub fn start(address: &str, model: Arc<Mutex<InMemoryModel>>) -> Result<(), ()> {
    let address_str = "http://".to_string() + &address + "/"; 
    let server = Server::http(address).map_err(|err| {
        eprintln!("{}: Could not create initiate server at {address} as {err}", "ERROR".bold().red(), address = address.bold().bright_blue(), err = err.to_string().red());
        exit(1);
    }).unwrap();

    println!("{info}: Server Listening at: {address}", info = "INFO".bright_cyan(), address = address_str.cyan());

    for request in server.incoming_requests() {
        serve_request(request, Arc::clone(&model)).map_err(|err| {
            eprintln!("{}: Failed to serve the request as {err}", "ERROR".bold().red(), err = err.to_string().red());
        }).ok(); // <- Don't stop here continue serving requests
    }
    eprintln!("{}: Server socket has shutdown", "ERROR".bold().red());
    Ok(())
}
    