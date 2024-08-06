use std::result::Result;

#[derive(Debug, PartialEq)]
pub struct TagBlock {
    pub receiver_timestamp: Option<u64>,
    pub destination_station: Option<String>,
    pub line_count: Option<u32>,
    pub relative_time: Option<u32>,
    pub source_station: Option<String>,
    pub text: Option<String>,
    pub checksum: u8,
}

impl TagBlock {
    pub fn parse(input: &str) -> Result<Option<Self>, String> {
        // Remove leading and trailing backslashes
        let input = input.trim_matches('\\');
        let parts: Vec<&str> = input.split('*').collect();

        if parts.len() != 2 {
            return Err("Invalid tag block format; missing checksum".into());
        }

        //let key_value_part = format!("{}{}",parts[0],'*');
        let key_value_part = parts[0];
        print!("part 0 {}", key_value_part);
        let checksum_str = parts[1];

        // Ensure checksum string length is 2
        if checksum_str.len() != 2 {
            return Err("Invalid checksum format".into());
        }

        // Parse the provided checksum
        let provided_checksum = u8::from_str_radix(checksum_str, 16)
            .map_err(|_| "Invalid checksum format".to_string())?;

        // Calculate the checksum
        let calculated_checksum = calculate_checksum(key_value_part.as_bytes());

        if calculated_checksum != provided_checksum {
            return Err(format!(
                "Checksum mismatch: calculated {:#02X}, expected {:#02X}",
                calculated_checksum, provided_checksum
            ));
        }

        let mut tag_block = TagBlock {
            receiver_timestamp: None,
            destination_station: None,
            line_count: None,
            relative_time: None,
            source_station: None,
            text: None,
            checksum: provided_checksum,
        };

        // Parse key-value pairs
        for kv in key_value_part.split(',') {
            if kv.len() < 3 {
                continue;
            }

            let (key, value) = kv.split_at(2);
            let value = value.to_string();

            match key {
                "c:" => {
                    tag_block.receiver_timestamp = value.parse().ok();
                }
                "d:" => {
                    tag_block.destination_station = Some(value);
                }
                "n:" => {
                    tag_block.line_count = value.parse().ok();
                }
                "r:" => {
                    tag_block.relative_time = value.parse().ok();
                }
                "s:" => {
                    tag_block.source_station = Some(value);
                }
                "t:" => {
                    tag_block.text = Some(value);
                }
                _ => {
                    // Ignore unknown keys
                }
            }
        }

        Ok(Some(tag_block))
    }
}

/// Calculates the checksum for the provided data using XOR operation
fn calculate_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &item| acc ^ item)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_block_with_valid_data() {
        let input = r"\s:1234567,c:1625140800*34\";
        let tag_block = TagBlock::parse(input).unwrap().unwrap();

        assert_eq!(tag_block.receiver_timestamp, Some(1671620143));
        assert_eq!(tag_block.source_station, Some("2573135".to_string()));
        assert_eq!(tag_block.checksum, 0x28);
        assert!(tag_block.destination_station.is_none());
        assert!(tag_block.line_count.is_none());
        assert!(tag_block.relative_time.is_none());
        assert!(tag_block.text.is_none());
    }

    #[test]
    fn parse_tag_block_with_all_fields() {
        let input = r"\s:2573135,c:1671620143,d:FooBar,n:123,r:456,t:HelloWorld!*03\";
        let tag_block = TagBlock::parse(input).unwrap().unwrap();

        assert_eq!(tag_block.receiver_timestamp, Some(1671620143));
        assert_eq!(tag_block.source_station, Some("2573135".to_string()));
        assert_eq!(tag_block.destination_station, Some("FooBar".to_string()));
        assert_eq!(tag_block.line_count, Some(123));
        assert_eq!(tag_block.relative_time, Some(456));
        assert_eq!(tag_block.text, Some("HelloWorld!".to_string()));
        assert_eq!(tag_block.checksum, 0x03);
    }

    #[test]
    fn parse_tag_block_with_invalid_format() {
        let input = r"invalid_tag_block_format";
        let result = TagBlock::parse(input);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            "Invalid tag block format; missing checksum"
        );
    }

    #[test]
    fn parse_tag_block_with_invalid_checksum_format() {
        let input = r"\s:2573135,c:1671620143*GG\";
        let result = TagBlock::parse(input);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            "Invalid checksum format"
        );
    }

    #[test]
    fn parse_tag_block_with_checksum_mismatch() {
        let input = r"\s:2573135,c:1671620143*FF\";
        let result = TagBlock::parse(input);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Checksum mismatch"));
    }

    #[test]
    fn parse_tag_block_with_unknown_keys() {
        let input = r"\x:unknown_key,s:2573135,c:1671620143*01\";
        let tag_block = TagBlock::parse(input).unwrap().unwrap();

        assert_eq!(tag_block.receiver_timestamp, Some(1671620143));
        assert_eq!(tag_block.source_station, Some("2573135".to_string()));
        assert_eq!(tag_block.checksum, 0x01);
        assert!(tag_block.destination_station.is_none());
        assert!(tag_block.line_count.is_none());
        assert!(tag_block.relative_time.is_none());
        assert!(tag_block.text.is_none());
    }
}