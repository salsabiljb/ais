use crate::errors::Error;
use crate::messages::AisMessage;
use crate::sentence::{AisFragments, AisParser};
use std::error::Error as StdError;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpStream, UdpSocket};

// Function to parse NMEA line
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

// Decodes a stream of ais messages from a UDP
pub async fn decode_from_udp(address: &str) -> Result<(), Box<dyn StdError>> {
    let socket = UdpSocket::bind(address).await?;
    let mut buf = [0; 1024];
    let mut parser = AisParser::new();

    loop {
        let (len, _) = socket.recv_from(&mut buf).await?;
        parse_nmea_line(&mut parser, &buf[..len]).await;
    }
}

// Decodes a stream of ais messages from a TCP
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

// Decodes a file of ais messages
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::position_report::NavigationStatus;
    use tempfile;
    use tokio::io::AsyncWriteExt;
    use tokio::net::{TcpListener, UdpSocket};

    // Function to validate PositionReport messages
    fn validate_position_report(report: &crate::messages::position_report::PositionReport) {
        assert_eq!(report.message_type, 1);
        assert_eq!(report.mmsi, 367380120);
        assert_eq!(
            report.navigation_status,
            Some(NavigationStatus::UnderWayUsingEngine)
        );
        assert_eq!(report.speed_over_ground, Some(0.1));
        assert_eq!(report.longitude, Some(-122.404335));
        assert_eq!(report.latitude, Some(37.806946));
        assert_eq!(report.course_over_ground, Some(245.2));
        assert_eq!(report.timestamp, 59);
        assert!(report.raim);
    }
    #[tokio::test]
    async fn test_parse_nmea_line() {
        let mut parser = AisParser::new();
        let line = b"!AIVDM,1,1,,B,15NG6V0P01G?cFhE`R2IU?wn28R>,0*05";

        parse_nmea_line(&mut parser, line).await;

        if let Ok(AisFragments::Complete(sentence)) = parser.parse(line, true) {
            if let Some(AisMessage::PositionReport(ref report)) = sentence.message {
                validate_position_report(report);
            } else {
                panic!("Failed to parse message as PositionReport");
            }
        } else {
            panic!("Failed to parse NMEA line");
        }
    }

    #[tokio::test]
    async fn test_decode_from_udp() {
        let address = "127.0.0.1:12345";
        let test_data = b"!AIVDM,1,1,,B,15NG6V0P01G?cFhE`R2IU?wn28R>,0*05";

        let server_handle = tokio::spawn(async move {
            decode_from_udp(address).await.unwrap();
        });

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(test_data, address).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut parser = AisParser::new();
        if let Ok(AisFragments::Complete(sentence)) = parser.parse(test_data, true) {
            if let Some(AisMessage::PositionReport(ref report)) = sentence.message {
                validate_position_report(report);
            } else {
                panic!("Failed to parse message as PositionReport");
            }
        } else {
            panic!("Failed to parse NMEA line");
        }

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_decode_from_tcp() {
        let address = "127.0.0.1:12346";
        let listener = TcpListener::bind(address).await.unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let test_data = b"!AIVDM,1,1,,B,15NG6V0P01G?cFhE`R2IU?wn28R>,0*05\n";
            socket.write_all(test_data).await.unwrap();
        });

        decode_from_tcp(address).await.unwrap();

        let message = b"!AIVDM,1,1,,B,15NG6V0P01G?cFhE`R2IU?wn28R>,0*05";
        let mut parser = AisParser::new();
        if let Ok(AisFragments::Complete(sentence)) = parser.parse(message, true) {
            if let Some(AisMessage::PositionReport(ref report)) = sentence.message {
                validate_position_report(report);
            } else {
                panic!("Failed to parse message as PositionReport");
            }
        } else {
            panic!("Failed to parse NMEA line");
        }
    }

    #[tokio::test]
    async fn test_decode_from_file() {
        let test_data = b"!AIVDM,1,1,,B,15NG6V0P01G?cFhE`R2IU?wn28R>,0*05\n";
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        tokio::fs::write(&file_path, test_data).await.unwrap();

        decode_from_file(file_path.to_str().unwrap()).await.unwrap();

        let mut parser = AisParser::new();
        if let Ok(AisFragments::Complete(sentence)) = parser.parse(test_data, true) {
            if let Some(AisMessage::PositionReport(ref report)) = sentence.message {
                validate_position_report(report);
            } else {
                panic!("Failed to parse message as PositionReport");
            }
        } else {
            panic!("Failed to parse NMEA line");
        }
    }

    #[test]
    fn test_decode() {
        let message = b"!AIVDM,1,1,,B,15NG6V0P01G?cFhE`R2IU?wn28R>,0*05";
        let result = decode(message);

        match result {
            Ok(AisMessage::PositionReport(ref report)) => {
                validate_position_report(report);
            }
            _ => panic!("Failed to decode the message correctly"),
        }
    }
}
