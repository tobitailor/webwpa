use std::io::ErrorKind;
use std::net::TcpListener;
use std::thread::{ spawn, sleep };
use std::time::Duration;

#[cfg(feature = "webserver")]
use std::str::FromStr;

use tungstenite::accept;
use tungstenite::error::Error;
use tungstenite::protocol::Message;
use wpactrl::Client;

#[cfg(feature = "webserver")]
use tiny_http::{ Header, Response, Server };

#[cfg(feature = "webserver")]
fn run_webserver() {
    let server = Server::http("0.0.0.0:80").unwrap();
    for request in server.incoming_requests() {
        let mut response = Response::from_string(include_str!("../www/webwpa.html"));
        response.add_header(Header::from_str("Content-Type: text/html").unwrap());
        request.respond(response).unwrap();
    }
}

fn main() {
    #[cfg(feature = "webserver")]
    spawn(run_webserver);

    let server = TcpListener::bind("0.0.0.0:81").unwrap();
    for stream in server.incoming() {
        spawn(move || {
            let stream = stream.unwrap();
            let mut ws = accept(stream.try_clone().unwrap()).unwrap();
            stream.set_nonblocking(true).unwrap();
            let client = Client::builder().open().unwrap();
            let mut wpa = client.attach().unwrap();
            loop {
                match ws.read() {
                    Ok(msg) => {
                        match msg {
                            Message::Text(txt) => {
                                match wpa.request(&txt) {
                                    Ok(res) => {
                                        let cmd = txt.split(' ').next().unwrap();
                                        let txt = format!("CTRL-RSP-{} {}", cmd, res);
                                        if let Err(err) = ws.send(Message::Text(txt)) {
                                            panic!("Failed sending message: {:?}", err);
                                        }
                                    },
                                    Err(err) => panic!("Failed sending command: {:?}", err),
                                }
                            },
                            Message::Close(_) => break,
                            _ => (),
                        }
                    },
                    Err(Error::Io(err)) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(err) => panic!("Failed reading message: {:?}", err),
                }
                match wpa.recv() {
                    Ok(msg) => {
                        if let Some(txt) = msg {
                            if let Err(err) = ws.send(Message::Text(txt)) {
                                panic!("Failed sending message: {:?}", err);
                            }
                        }
                    },
                    Err(err) => panic!("Failed receiving message: {:?}", err),
                }
                sleep(Duration::from_millis(10));
            }
        });
    }
}
