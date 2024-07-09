use json_parser_simple::{json_scan, JsonValue};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

struct Peers {
    ip_port: String,
    stream: TcpStream,
}

impl Peers {
    fn new(ip_port: String, stream: TcpStream) -> Self {
        Self { ip_port, stream }
    }
}

impl Clone for Peers {
    fn clone(&self) -> Self {
        Self {
            ip_port: self.ip_port.clone(),
            stream: self.stream.try_clone().expect("Failed to clone stream"),
        }
    }
}

fn handle(mut stream: TcpStream, bounds: Arc<Mutex<HashMap<i32, Peers>>>) {
    let mut buffer = [0; 512];
    while let Ok(size) = stream.read(&mut buffer) {
        if size == 0 {
            break;
        }

        let message = String::from_utf8_lossy(&buffer[..size]);
        if let Some(index) = message.find('}') {
            let json_str = &message[..=index];
            let json_map = json_scan(json_str);
            if let Some(JsonValue::String(msg)) = json_map.get("msg") {
                if let Some(JsonValue::Number(id)) = json_map.get("id") {
                    if let Some(JsonValue::String(ip)) = json_map.get("ip") {
                        if msg == "RECONNECT"{
                            let id = *id as i32;
                            let mut bounds = bounds.lock().unwrap();
                            if bounds.contains_key(&id) {
                                let current_ip = bounds.get_mut(&id).unwrap().ip_port.clone();
                                let mut response = format!(r#"{{"msg":"CONNECT","ip":"{}"}}"#, current_ip);
                               
                                if let Err(e) = stream.write_all(response.as_bytes()) {
                                    println!("Failed to write to stream: {}", e);
                                }
                                response = format!(r#"{{"msg":"CONNECT","ip":"{}"}}"#, ip);
                                
                                if let Err(e) = bounds.get_mut(&id).unwrap().stream.write_all(response.as_bytes()) {
                                    println!("Failed to write to stream: {}", e);
                                }
                                bounds.remove(&id);
                            } else {
                                let peer = Peers {
                                    ip_port: ip.clone(),
                                    stream: stream.try_clone().expect("Failed to clone stream"),
                                };
                                bounds.insert(id, peer);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn on_dis(ip_port: &String) {
    let bounds = Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind(ip_port).expect("Could not bind");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let bounds = Arc::clone(&bounds);
                let handle_stream = stream.try_clone().expect("Failed to clone stream");
                thread::spawn(move || {
                    handle(handle_stream, bounds);
                });
            }
            Err(e) => {
                println!("Connection failed: {}", e);
            }
        }
    }
}
