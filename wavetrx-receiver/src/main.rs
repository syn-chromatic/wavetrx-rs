mod receiver;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    receiver::live_output_receiver()?;
    Ok(())
}
