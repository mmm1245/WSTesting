use std::fs::{File};
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use tungstenite::{connect, Message, WebSocket};
use tungstenite::stream::MaybeTlsStream;
use url::Url;
use std::env;
use std::path::PathBuf;
use glob::{glob_with, MatchOptions};
use json::JsonValue;
use regex::Regex;

fn is_path_file(path : &PathBuf) -> bool {
    let meta = path.metadata();
    if let Ok(meta) = meta {
        return meta.is_file();
    }
    false
}

fn main() {
    let mut args = env::args();
    let ws_url = Url::parse(args.nth(1).unwrap().as_str()).unwrap();
    let glob_pattern = args.next().unwrap();

    let glob_options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    let mut test_results : Vec<(String,Result<(),String>)> = Vec::new();
    for entry in glob_with(glob_pattern.as_str(), glob_options).expect("Invalid glob pattern") {
        if let Ok(path) = entry {
            if !is_path_file(&path) {
                continue;
            }
            let file = File::open(&path);
            let test_name = path.display().to_string();
            match file {
                Ok(file) => {
                    test_results.push((test_name,run_test(ws_url.clone(), file)));
                }
                Err(_) => {
                    test_results.push((test_name.clone(),Err(format!("Could not open test file {}", test_name))));
                }
            }
        }
    }
    println!("{}/{} tests passed", test_results.iter().filter(|e| e.1.is_ok()).count(), test_results.len());
    for result in test_results {
        if let Err(reason) = result.1 {
            println!("{} failed with: {}", result.0, reason);
        }
    }
}

fn run_test(url : Url, file : File) -> Result<(),String>{
    let (mut socket, _) = connect(url).map_err(|_| "Can't connect")?;
    let file = BufReader::new(file);
    for line in file.lines() {
        if let Ok(line) = line {
            let mut chars = line.chars();
            let cmd = chars.next().ok_or("Line cannot be empty")?;
            let sep = chars.next().ok_or("Separator not found")?;
            if sep != ':' {
                return Err("Separator should be ':'".to_string());
            }
            let data = chars.as_str();
            match cmd {
                'S' => {
                    socket.write_message(Message::Text(data.to_string())).map_err(|_|"Couldnt send message(probably closed)")?;
                }
                'E' => {
                    let result = read_text(& mut socket)?;
                    if result != data {
                        return Err(format!("\n\texpected {} \n\tgot {}", data, result));
                    }
                }
                'R' => {
                    let regex = Regex::new(data).map_err(|_|"Invalid regex")?;
                    let result = read_text(& mut socket)?;
                    if !regex.is_match(result.as_str()) {
                        return Err(format!("\n\t{} doesnt match\n\tregex {}", result, data));
                    }
                }
                'J' => {
                    let result = read_text(& mut socket)?;
                    let data_json = json::parse(data).map_err(|_| format!("Could not parse test json {}", data))?;
                    let text_json = json::parse(result.as_str()).map_err(|_| format!("Could not parse read json {}", result))?;
                    if !does_json_include(&text_json, &data_json) {
                        return Err(format!("\n\t{} doesnt include\n\tjson {}", result, data));
                    }
                }
                '#' => {}
                _ => {
                    return Err(format!("Unknown cmd {}", cmd));
                }
            }
        }
    }
    Ok(())
}

fn does_json_include(input : &JsonValue, expected : &JsonValue) -> bool {
    return match expected.clone() {
        JsonValue::Object(expect_obj) => {
            if let JsonValue::Object(input_obj) = input {
                for x in expect_obj.iter() {
                    if !does_json_include(&input_obj[x.0], x.1) {
                        return false;
                    }
                }
                return true;
            }
            false
        }
        JsonValue::Array(expect_arr) => {
            if let JsonValue::Array(input_arr) = input {
                let mut iter = input_arr.iter();
                for e in expect_arr.iter() {
                    let input_val = iter.next();
                    if let Some(input_val) = input_val {
                        if !does_json_include(input_val, e) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                return true;
            }
            false
        }
        _ => {
            expected == input
        }
    }
}

fn read_text(socket : &mut WebSocket<MaybeTlsStream<TcpStream>>) -> Result<String,String> {
    loop {
        let read_val = match socket.read_message().map_err(|_| "Couldn't read message")? {
            Message::Text(text) => {Some(Ok(text))}
            Message::Binary(_) => {Some(Err("Binary frames not supported"))}
            Message::Ping(_) => {None}
            Message::Pong(_) => {None}
            Message::Close(_) => {Some(Err("Websocket closed"))}
            Message::Frame(_) => unreachable!()
        };
        if let Some(read_val) = read_val {
            return read_val.map_err(|err| err.to_string())
        }
    }
}