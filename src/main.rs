use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::mpsc::channel,
    thread,
};

use clap::Parser;

use crate::chatroom::Chatroom;

mod chatroom;

#[derive(Parser)]
struct Args {
    /// bind the service to this tcp port, default 5555
    #[arg(short, long, default_value = "5555")]
    port: u16,
}

fn main() {
    let args = Args::parse();
    let s = format!("0.0.0.0:{}", args.port)
        .parse::<SocketAddr>()
        .unwrap();
    println!("Listening to {s}");
    let chatroom = Chatroom::default();
    let listener = TcpListener::bind(s).unwrap();
    for incoming in listener.incoming() {
        match incoming {
            Ok(incoming) => {
                let chatroom = chatroom.clone();
                thread::spawn(|| chat(incoming, chatroom));
            }

            Err(e) => eprintln!("error {e}"),
        }
    }
}

fn chat(mut stream: TcpStream, chatroom: Chatroom) {
    let peer_addr = stream.peer_addr().unwrap();

    println!("{peer_addr} - connected!");

    let mut read_stream = BufReader::new(stream.try_clone().unwrap());

    stream
        .write_all(b"Welcome to our chat room, please enter your nickname:\n")
        .unwrap();

    let mut nickname = String::new();
    read_stream.read_line(&mut nickname).unwrap();

    let (sender, receiver) = channel();

    let nickname = nickname.trim().to_string();
    match chatroom.join(nickname.clone(), sender) {
        Ok(session) => {
            thread::spawn(move || {
                for message in receiver.iter() {
                    let _ = write!(stream, "{message}\n");
                }
            });
            for line in read_stream.lines() {
                if let Ok(line) = line {
                    let line = line.trim().to_string();
                    session.send_message(line);
                } else {
                    break;
                }
            }
            // Note: session will be dropped thus, the user will leave the chatroom
        }
        Err(e) => {
            write!(stream, "{e}\n").unwrap();
        }
    }

    println!("{peer_addr} - connection ended");
}
