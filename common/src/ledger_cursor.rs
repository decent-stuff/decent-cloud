use std::collections::HashMap;

use crate::error;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;

/// Enum defining the direction of the cursor.
/// Future implementations may support backward cursors.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum CursorDirection {
    #[default]
    Forward,
    Backward,
}

impl std::fmt::Display for CursorDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CursorDirection::Forward => write!(f, "forward"),
            CursorDirection::Backward => write!(f, "backward"),
        }
    }
}

impl std::str::FromStr for CursorDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "forward" => Ok(CursorDirection::Forward),
            "backward" => Ok(CursorDirection::Backward),
            _ => Err(format!("Invalid direction: {}", s)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LedgerCursor {
    pub data_begin_position: u64,
    pub position: u64,
    pub data_end_position: u64,
    pub response_bytes: u64,
    pub direction: CursorDirection,
    pub more: bool,
}

impl std::fmt::Display for LedgerCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(position 0x{:0x}) {}",
            self.position,
            self.to_urlenc_string()
        )
    }
}

impl std::str::FromStr for LedgerCursor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut data_begin_position = None;
        let mut position = None;
        let mut end_position = None;
        let mut response_bytes = None;
        let mut direction = None;
        let mut more = None;

        for part in s.split('&') {
            let (key, value) = match part.split_once('=') {
                Some(x) => x,
                None => {
                    error!("Skipping invalid part {} of input {}", part, s);
                    continue;
                }
            };

            match key {
                "data_begin_position" => {
                    data_begin_position =
                        Some(value.parse().map_err(|_| "Invalid begin_position value")?);
                }
                "position" => {
                    position = Some(value.parse().map_err(|_| "Invalid position value")?);
                }
                "data_end_position" => {
                    end_position = Some(value.parse().map_err(|_| "Invalid end_position value")?);
                }
                "response_bytes" => {
                    response_bytes =
                        Some(value.parse().map_err(|_| "Invalid response_bytes value")?);
                }
                "direction" => {
                    direction = Some(value.parse().map_err(|_| "Invalid direction value")?);
                }
                "more" => {
                    more = Some(value.parse().map_err(|_| "Invalid more value")?);
                }
                _ => return Err(format!("Unexpected key: {}", key)),
            }
        }

        Ok(LedgerCursor {
            data_begin_position: data_begin_position.unwrap_or_default(),
            position: position.unwrap_or_default(),
            data_end_position: end_position.unwrap_or_default(),
            response_bytes: response_bytes.unwrap_or_default(),
            direction: direction.unwrap_or_default(),
            more: more.unwrap_or_default(),
        })
    }
}

impl From<HashMap<String, MetadataValue>> for LedgerCursor {
    fn from(value: HashMap<String, MetadataValue>) -> Self {
        let mut data_begin_position = 0;
        let mut data_end_position = 0;
        for (key, value) in value.into_iter() {
            match value {
                MetadataValue::Nat(nat) if key == "ledger:data_start_lba" => {
                    data_begin_position = nat.0.to_u64_digits()[0];
                }
                MetadataValue::Nat(nat) if key == "ledger:next_block_write_position" => {
                    data_end_position = nat.0.to_u64_digits()[0];
                }
                _ => continue,
            }
        }

        Self {
            data_begin_position,
            position: 0,
            data_end_position,
            response_bytes: 0,
            direction: CursorDirection::Forward,
            more: false,
        }
    }
}

impl LedgerCursor {
    pub fn new(
        data_begin_position: u64,
        position: u64,
        data_end_position: u64,
        direction: CursorDirection,
        has_more: bool,
    ) -> Self {
        Self {
            data_begin_position,
            position,
            data_end_position,
            response_bytes: 0,
            direction,
            more: has_more,
        }
    }

    pub fn new_from_string(s: String) -> Result<Self, String> {
        s.parse()
    }

    pub fn to_request_string(&self) -> String {
        format!("position={}", self.position)
    }

    pub fn to_urlenc_string(&self) -> String {
        format!(
            "position={}&response_bytes={}&direction={}&more={}",
            self.position, self.response_bytes, self.direction, self.more
        )
    }
}

/// loc_ledger_start_data_lba: The first LBA in the persistent storage.
/// loc_storage_bytes: How many bytes are allocated in the persistent storage?
///                    Note that not all allocated bytes may be in use.
/// loc_next_write_position: The next position in the persistent storage that has not been written.
///                    Positions before may hold data, positions after do not.
/// req_start_position: The position requested by the user.
pub fn cursor_from_data(
    loc_ledger_start_data_lba: u64,
    loc_storage_bytes: u64,
    loc_next_write_position: u64,
    req_start_position: u64,
) -> LedgerCursor {
    // Handle edge case: req_start_position is before the start of the data partition
    let response_start_position = loc_ledger_start_data_lba.max(req_start_position);

    // Handle edge case: loc_next_write_position is beyond the end of the persistent storage
    let loc_next_write_position = loc_next_write_position.min(loc_storage_bytes);

    // Start - end position ==> size
    // size is ideally equal to FETCH_SIZE_BYTES_DEFAULT, but there may not be enough data in the persistent storage
    let response_end_position =
        (response_start_position + crate::FETCH_SIZE_BYTES_DEFAULT).min(loc_next_write_position);

    if response_start_position >= loc_storage_bytes
        || response_start_position >= response_end_position
    {
        return LedgerCursor {
            data_begin_position: loc_ledger_start_data_lba,
            position: loc_next_write_position,
            data_end_position: loc_next_write_position,
            response_bytes: 0,
            direction: CursorDirection::Forward,
            more: false,
        };
    }

    LedgerCursor {
        data_begin_position: loc_ledger_start_data_lba,
        position: response_start_position,
        data_end_position: loc_next_write_position,
        response_bytes: response_end_position - response_start_position,
        direction: CursorDirection::Forward,
        more: response_end_position < loc_next_write_position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ledger_cursor() {
        let input = "data_begin_position=0&position=123&data_end_position=579&response_bytes=456&direction=Forward&more=true";
        let cursor: LedgerCursor = input.parse()
            .expect("Failed to parse valid cursor string in test");

        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 123,
                data_end_position: 123 + 456,
                response_bytes: 456,
                direction: CursorDirection::Forward,
                more: true,
            }
        );
    }

    #[test]
    fn test_parse_ledger_cursor_invalid_format() {
        let input = "position=123&response_bytes=456&direction=Invalid&more=true";
        let cursor: Result<LedgerCursor, _> = input.parse();
        assert!(cursor.is_err());
    }

    #[test]
    fn test_parse_ledger_cursor_missing_key() {
        let cursor = LedgerCursor::new_from_string(
            "position=123&data_end_position=579&response_bytes=456&more=true".to_string(),
        )
        .expect("Failed to parse valid cursor with missing optional keys");
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 123,
                data_end_position: 123 + 456,
                response_bytes: 456,
                direction: CursorDirection::Forward,
                more: true,
            }
        )
    }

    #[test]
    fn test_parse_ledger_cursor_invalid_value() {
        let input = "position=123&response_bytes=abc&direction=Forward&more=true";
        let cursor: Result<LedgerCursor, _> = input.parse();
        assert!(cursor.is_err());
    }

    #[test]
    fn test_cursor_no_data() {
        let cursor = cursor_from_data(0, 0, 0, 1000);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 0,
                data_end_position: 0,
                response_bytes: 0,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_within_bounds() {
        let cursor = cursor_from_data(1024, 4096, 2048, 1024);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 1024,
                position: 1024,
                data_end_position: 2048,
                response_bytes: 2048 - 1024,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_exceeds_bounds() {
        let cursor = cursor_from_data(0, 2048, 1500, 0);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 0,
                data_end_position: 1500,
                response_bytes: 1500,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_request_start_beyond_next_write() {
        let cursor = cursor_from_data(0, 2048, 1024, 2048);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 1024,
                data_end_position: 1024,
                response_bytes: 0,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_request_start_beyond_storage() {
        let cursor = cursor_from_data(0, 1024, 1024, 2048);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 1024,
                data_end_position: 1024,
                response_bytes: 0,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_start_position_adjusted() {
        let cursor = cursor_from_data(512, 2048, 2048, 256);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 512,
                position: 512,
                data_end_position: 2048,
                response_bytes: 2048 - 512,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_no_data_available() {
        let cursor = cursor_from_data(0, 1024, 1024, 1024);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 1024,
                data_end_position: 1024,
                response_bytes: 0,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
        let cursor = cursor_from_data(65536, 1049113, 327680, 327680);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 65536,
                position: 327680,
                data_end_position: 327680,
                response_bytes: 0,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_partial_data_available() {
        let cursor: LedgerCursor = cursor_from_data(0, 2048, 1500, 512);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 0,
                position: 512,
                data_end_position: 1500,
                response_bytes: 988,
                direction: CursorDirection::Forward,
                more: false,
            }
        );

        let cursor: LedgerCursor = cursor_from_data(327680, 458752, 328217, 327680);
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 327680,
                position: 327680,
                data_end_position: 328217,
                response_bytes: 537,
                direction: CursorDirection::Forward,
                more: false,
            }
        );
    }

    #[test]
    fn test_cursor_full_data_available() {
        let cursor = cursor_from_data(65536, 1024 * 1024 * 1024, 1024 * 1024 * 1024 - 1234, 0);
        // first fetch
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 65536,
                position: 65536,
                data_end_position: 1024 * 1024 * 1024 - 1234,
                response_bytes: crate::FETCH_SIZE_BYTES_DEFAULT,
                direction: CursorDirection::Forward,
                more: true,
            }
        );
        // second fetch
        let cursor = cursor_from_data(
            65536,
            1024 * 1024 * 1024,
            1024 * 1024 * 1024 - 1234,
            crate::FETCH_SIZE_BYTES_DEFAULT,
        );
        assert_eq!(
            cursor,
            LedgerCursor {
                data_begin_position: 65536,
                position: crate::FETCH_SIZE_BYTES_DEFAULT,
                data_end_position: 1024 * 1024 * 1024 - 1234,
                response_bytes: crate::FETCH_SIZE_BYTES_DEFAULT,
                direction: CursorDirection::Forward,
                more: true,
            }
        );
    }
}
