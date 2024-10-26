use np_offering::Offering;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <YAML/JSON file> <Search string>", args[0]);
        std::process::exit(1);
    }

    let offering = Offering::new_from_file(&args[1]).expect("Failed to parse offering");
    if offering.matches_search(&args[2]) {
        eprintln!("Profile matches!");
        println!("{}", offering);
    } else {
        eprintln!("Profile does not match!");
        std::process::exit(1);
    }
}
