/// A simple checksum utility for validating data integrity using XOR.
pub struct CheckSum;

impl CheckSum {
    /// Computes a 16-bit XOR-based checksum over the given payload.
    ///
    /// # Arguments
    ///
    /// * `payload` - A byte slice containing the data to compute the checksum for.
    ///
    /// # Returns
    ///
    /// A `u16` representing the XOR checksum of the input payload.
    pub fn new(payload: &[u8]) -> u16 {
        let mut checksum: u16 = 0;
        for &byte in payload {
            checksum ^= byte as u16;
        }
        return checksum;
    }

    /// Verifies that the provided checksum matches the computed checksum for the payload.
    ///
    /// # Arguments
    ///
    /// * `checksum` - A reference to the expected checksum as `i16`.
    /// * `payload` - A byte slice containing the data to validate.
    ///
    /// # Returns
    ///
    /// `true` if the provided checksum matches the computed checksum; `false` otherwise.
    pub fn check(checksum: &i16, payload: &[u8]) -> bool {
        let check = CheckSum::new(payload);
        return *checksum == check as i16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_empty_payload() {
        let payload: &[u8] = &[];
        let expected: u16 = 0;
        assert_eq!(CheckSum::new(payload), expected);
    }

    #[test]
    fn test_checksum_single_byte() {
        let payload: &[u8] = &[0xAB];
        let expected: u16 = 0xAB;
        assert_eq!(CheckSum::new(payload), expected);
    }

    #[test]
    fn test_checksum_multiple_bytes() {
        let payload: &[u8] = &[0x01, 0x02, 0x03];
        // XOR: 0x01 ^ 0x02 = 0x03, 0x03 ^ 0x03 = 0x00
        let expected: u16 = 0x00;
        assert_eq!(CheckSum::new(payload), expected);
    }

    #[test]
    fn test_checksum_check_valid() {
        let payload: &[u8] = &[0x10, 0x20, 0x30];
        let checksum = CheckSum::new(payload) as i16;
        assert!(CheckSum::check(&checksum, payload));
    }

    #[test]
    fn test_checksum_check_invalid() {
        let payload: &[u8] = &[0x10, 0x20, 0x30];
        let bad_checksum: i16 = 0xFF;
        assert!(!CheckSum::check(&bad_checksum, payload));
    }
}
