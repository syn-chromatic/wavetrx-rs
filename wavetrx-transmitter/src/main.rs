mod tests;
use crate::tests::test_transmitter_player;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    test_transmitter_player()?;
    Ok(())
}
