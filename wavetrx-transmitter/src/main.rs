mod transmitter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    transmitter::transmitter_player()?;
    Ok(())
}
