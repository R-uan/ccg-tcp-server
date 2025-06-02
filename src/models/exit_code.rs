#[derive(Default)]
pub struct ExitStatus {
    pub code: i32,
    pub reason: String,
}

#[repr(i32)]
pub enum ExitCode {
    MatchEnded = 00,
    
    CardRequestFailed = 10,
}