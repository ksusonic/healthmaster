use healthmaster::app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let err = app::run_default();
    if let Err(e) = err {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
