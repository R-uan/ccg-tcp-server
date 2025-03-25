pub struct CheckSum;

impl CheckSum {
    pub fn new(payload: &[u8]) -> u16 {
        let mut checksum: u16 = 0;
        for &byte in payload {
            checksum ^= byte as u16;
        }
        return checksum;
    }

    pub fn check(checksum: &i16, payload: &[u8]) -> bool {
        let check = CheckSum::new(payload);
        println!("> {:?}", payload);
        return *checksum == check as i16;
    }
}
