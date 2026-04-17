fn main() {
    if let Err(error) = hearthsync::run_cli() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
