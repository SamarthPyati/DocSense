use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use std::{
    self,
    cmp::Ordering,
    fs::{self, File},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    process::exit,
    str,
    sync::{Arc, Mutex}
}; 

use colored::Colorize;
use crate::RankMethod;

use super::model::*;

pub fn serve_status_code(request: Request, statuscode: i32) -> io::Result<()> {
    return request.respond(Response::from_string(statuscode.to_string()).with_status_code(StatusCode(statuscode as u16)));
}

pub fn serve_404(request: Request) -> io::Result<()> {
    return serve_status_code(request, 404);
}


pub fn serve_500(request: Request) -> io::Result<()> {
    return serve_status_code(request, 500);
}


pub fn serve_400(request: Request, message: &str) -> io::Result<()> {
    return request.respond(Response::from_string(format!("400: {message}")).with_status_code(StatusCode(400)));
}


pub fn serve_static_file(request: Request, file_path: &str, content_type: &str) -> io::Result<()> {
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
    let header = Header::from_bytes("Content-Type", content_type).expect("Should be a valid Content-Type while passing the header.");
    return request.respond(Response::from_file(html_file).with_header(header));
}


pub fn serve_api_search(mut request: Request, model: Arc<Mutex<InMemoryModel>>, rank_method: RankMethod) -> io::Result<()>{
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
    let results = match model.search_query(&body, &model, rank_method) {
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

fn from_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(encoded: &str) -> Result<String, ()> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                decoded.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = from_hex_digit(bytes[i + 1]);
                let lo = from_hex_digit(bytes[i + 2]);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    decoded.push(hi << 4 | lo);
                    i += 3;
                } else {
                    return Err(());
                }
            }
            b => {
                decoded.push(b);
                i += 1;
            }
        }
    }

    String::from_utf8(decoded).map_err(|_| ())
}

fn extract_query_param(url: &str, name: &str) -> Option<String> {
    let query_start = url.find('?')?;
    for pair in url[query_start + 1..].split('&') {
        let mut parts = pair.splitn(2, '=');
        if let Some(key) = parts.next() {
            if key == name {
                return percent_decode(parts.next().unwrap_or("")) .ok();
            }
        }
    }
    None
}

fn content_type_for_file_path(path: &Path) -> &str {
    let extension = path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_lowercase());
    match extension.as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("xhtml") => "application/xhtml+xml; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        Some("md") => "text/markdown; charset=utf-8",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn serve_file_request(request: Request, url: &str, root_dir: &Path) -> io::Result<()> {
    let path_query = match extract_query_param(url, "path") {
        Some(value) => value,
        None => return serve_400(request, "Missing file path query parameter"),
    };

    let requested_path = Path::new(&path_query);
    let normalized_path = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        root_dir.join(requested_path)
    };

    let canonicalized = match fs::canonicalize(&normalized_path) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}: Could not canonicalize requested file path {requested_path} as {err}", "ERROR".bold().red(), requested_path = normalized_path.to_string_lossy().bright_blue(), err = err.to_string().red());
            return serve_404(request);
        }
    };

    if !canonicalized.starts_with(root_dir) || !canonicalized.is_file() {
        return serve_404(request);
    }

    let content_type = content_type_for_file_path(&canonicalized);
    serve_static_file(request, canonicalized.to_str().unwrap_or_default(), content_type)
}

pub fn serve_request(request: Request, model: Arc<Mutex<InMemoryModel>>, rank_method: RankMethod, root_dir: &Path) -> io::Result<()> {
    let request_url = request.url().to_string();
    println!("{info}: Received request! method: [{req}], url: {url:?}",
        info = "INFO".bright_cyan(), 
        req = &request.method().as_str().bright_green(),
        url = &request_url
    );

    match (&request.method(), request_url.as_str()) {
        
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "src/index.html", "text/html; charset=utf-8")?
        }

        (Method::Get, "/index.js") => {
            serve_static_file(request, "src/index.js", "text/javascript; charset=utf-8")?
        }

        (Method::Get, url) if url.starts_with("/file") => {
            serve_file_request(request, url, root_dir)?
        }

        (Method::Post, "/api/search") => {
            serve_api_search(request, model, rank_method)?
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


pub fn start(address: &str, model: Arc<Mutex<InMemoryModel>>, rank_method: RankMethod, root_dir: PathBuf) -> Result<(), ()> {
    let address_str = "http://".to_string() + &address + "/"; 
    let server = Server::http(address).map_err(|err| {
        eprintln!("{}: Could not create initiate server at {address} as {err}", "ERROR".bold().red(), address = address.bold().bright_blue(), err = err.to_string().red());
        exit(1);
    }).unwrap();

    println!("{info}: Server Listening at: {address}", info = "INFO".bright_cyan(), address = address_str.cyan());

    for request in server.incoming_requests() {
        serve_request(request, Arc::clone(&model), rank_method.clone(), &root_dir).map_err(|err| {
            eprintln!("{}: Failed to serve the request as {err}", "ERROR".bold().red(), err = err.to_string().red());
        }).ok(); // <- Don't stop here continue serving requests
    }
    eprintln!("{}: Server socket has shutdown", "ERROR".bold().red());
    Ok(())
}
    