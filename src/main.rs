use std::{env, path::PathBuf};

use anyhow::Context;
use csv::Trim;
use repository::ClientRepository;
use std::{fs::File, io::BufReader};
use transaction::Transaction;

use anyhow::{Ok, Result};

mod client;
mod repository;
mod transaction;

fn main() -> Result<()> {
    env_logger::init();

    // First argument only
    let path: PathBuf = env::args()
        .nth(1)
        .context("Expected at least one argument")?
        .into();

    let mut client_repository = ClientRepository::new();

    // Set up csv reader
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = ::csv::ReaderBuilder::new()
        .flexible(true)
        // Trim all whitespace
        .trim(Trim::All)
        .from_reader(reader);

    // Read line by line and process
    for line in csv_reader.deserialize() {
        let transaction: Transaction = line?;
        if let Err(e) = client_repository.process(transaction) {
            log::error!("{e}");
        }
    }

    client_repository.output()?;

    Ok(())
}
