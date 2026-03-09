use clap::Parser;
use healthmaster::app;

#[derive(Parser)]
#[command(name = "healthmaster")]
#[command(about = "A health monitoring service", long_about = None)]
struct Args {
    #[arg(short, long, default_value = app::DEFAULT_CONFIG_PATH)]
    config: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let err = app::run(&args.config);
    if let Err(e) = err {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
