mod tests;
use crate::tests::test_live_recording_receiver3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    test_live_recording_receiver3()?;
    Ok(())
}
