/// A simple checksum utility for validating data integrity using XOR.
pub struct Checksum;

impl Checksum {
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
        // Iterate over each byte in the payload
        for &byte in payload {
            // XOR the current byte with the checksum
            checksum ^= byte as u16;
        }
        // Return the computed checksum
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
        // Compute the checksum for the given payload
        let check = Checksum::new(payload);
        // Compare the provided checksum with the computed checksum
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
        // Verify that the checksum for an empty payload is 0
        assert_eq!(Checksum::new(payload), expected);
    }

    #[test]
    fn test_checksum_single_byte() {
        let payload: &[u8] = &[0xAB];
        let expected: u16 = 0xAB;
        // Verify that the checksum for a single byte matches the byte value
        assert_eq!(Checksum::new(payload), expected);
    }

    #[test]
    fn test_checksum_multiple_bytes() {
        let payload: &[u8] = &[0x01, 0x02, 0x03];
        // XOR: 0x01 ^ 0x02 = 0x03, 0x03 ^ 0x03 = 0x00
        let expected: u16 = 0x00;
        // Verify that the checksum for multiple bytes is computed correctly
        assert_eq!(Checksum::new(payload), expected);
    }

    #[test]
    fn test_checksum_check_valid() {
        let payload: &[u8] = &[0x10, 0x20, 0x30];
        let checksum = Checksum::new(payload) as i16;
        // Verify that the checksum validation passes for a valid checksum
        assert!(Checksum::check(&checksum, payload));
    }

    #[test]
    fn test_checksum_check_invalid() {
        let payload: &[u8] = &[0x10, 0x20, 0x30];
        let bad_checksum: i16 = 0xFF;
        // Verify that the checksum validation fails for an invalid checksum
        assert!(!Checksum::check(&bad_checksum, payload));
    }
}
