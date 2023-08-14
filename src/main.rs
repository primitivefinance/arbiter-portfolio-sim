mod cli;
mod setup;
mod bindings;

/// # Proto Sim
/// Proof of concept simulation of EVM execution with an arbitrageur agent,
/// price process, "centralized" exchange, and the Portfolio protocol.
///
/// ## Overview
/// Executes the cli commands.
///
/// # Examples:
/// ```bash
/// cargo run sim
/// cargo run analyze -n trading_function -s error
/// cargo run analyze -n trading_function -s curve
/// ```
///
/// # Errors
/// - The `out_data` directory does not exist.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Grab the cli commands and execute them.
    let _ = cli::main().await?;

    Ok(())
}
