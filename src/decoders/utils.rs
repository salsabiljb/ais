use crate::errors::Error;
use crate::messages::AisMessage;
use crate::sentence::{AisFragments, AisParser};
use std::error::Error as StdError;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpStream, UdpSocket};

// Function to parse NMEA line and handle errors
async fn parse_nmea_line(parser: &mut AisParser, line: &[u8]) {
    match parser.parse(line, true) {
        Ok(sentence) => {
            if let AisFragments::Complete(sentence) = sentence {
                println!(
                    "{:?}\t{:?}",
                    std::str::from_utf8(line).unwrap(),
                    sentence.message
                );
            }
        }
        Err(err) => {
            eprintln!(
                "Error parsing line {:?}: {:?}",
                std::str::from_utf8(line).unwrap(),
                err
            );
        }
    }
}

pub async fn decode_from_udp(address: &str) -> Result<(), Box<dyn StdError>> {
    let socket = UdpSocket::bind(address).await?;
    let mut buf = [0; 1024];
    let mut parser = AisParser::new();

    loop {
        let (len, _) = socket.recv_from(&mut buf).await?;
        parse_nmea_line(&mut parser, &buf[..len]).await;
    }
}

pub async fn decode_from_tcp(address: &str) -> Result<(), Box<dyn StdError>> {
    let stream = TcpStream::connect(address).await?;
    let mut parser = AisParser::new();
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();

    while reader.read_until(b'\n', &mut line).await? != 0 {
        parse_nmea_line(&mut parser, &line).await;
        line.clear();
    }

    Ok(())
}

pub async fn decode_from_file(path: &str) -> Result<(), Box<dyn StdError>> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut parser = AisParser::new();

    while let Some(line) = lines.next_line().await? {
        parse_nmea_line(&mut parser, line.as_bytes()).await;
    }

    Ok(())
}

// Decodes a single message
pub fn decode(message: &[u8]) -> Result<AisMessage, Error> {
    let mut parser = AisParser::new();
    match parser.parse(message, true)? {
        AisFragments::Complete(sentence) => sentence.message.ok_or(Error::Nmea {
            msg: "Incomplete message".into(),
        }),
        _ => Err(Error::Nmea {
            msg: "Incomplete message".into(),
        }),
    }
}
