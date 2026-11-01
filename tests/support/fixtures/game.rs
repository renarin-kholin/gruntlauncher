fn main() {
    if cfg!(game_failure) {
        eprintln!("runtime failed");
        std::process::exit(23);
    }

    println!("{}", std::env::current_dir().unwrap().display());
    for argument in std::env::args().skip(1) {
        println!("{argument}");
    }
    eprintln!("runtime diagnostic");
}
