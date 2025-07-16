use clap::Args;
use std::error::Error;

#[derive(Args)]
pub struct TestEpisodeCommand {
    #[arg(short, long, default_value = "1")]
    pub participants: usize,
}

impl TestEpisodeCommand {
    pub async fn execute(self) -> Result<(), Box<dyn Error>> {
        println!("Running test episode with {} participants", self.participants);
        // Implementation would go here
        Ok(())
    }
}