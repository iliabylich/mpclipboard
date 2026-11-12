use std::io::{BufRead, Read, Write};
use std::net::TcpStream;

fn main() {
    let addr = std::env::var("MPCLIPBOARD_BIND_ADDRESS").unwrap();
    let token = std::env::var("MPCLIPBOARD_TOKEN").unwrap();
    let id = std::env::args().nth(1).unwrap();

    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "GET / HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: mpclipboard-raw\r\nToken: {token}\r\nID: {id}\r\nVersion: 6\r\n\r\n"
    )
    .unwrap();

    let mut response = Vec::new();
    while !response.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        response.push(byte[0]);
    }
    print!("{}", String::from_utf8(response).unwrap());

    let mut reader = stream.try_clone().unwrap();
    std::thread::spawn(move || {
        loop {
            let mut frame = [0; 256];
            reader.read_exact(&mut frame).unwrap();
            let len = usize::from(frame[0]);
            println!("< {}", std::str::from_utf8(&frame[1..=len]).unwrap());
        }
    });

    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let text = &line[..line.floor_char_boundary(255)];
        let mut frame = [0; 256];
        frame[0] = text.len() as u8;
        frame[1..=text.len()].copy_from_slice(text.as_bytes());
        stream.write_all(&frame).unwrap();
    }
}
