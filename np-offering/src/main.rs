use np_offering::Offering;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <YAML file> <Search string>", args[0]);
        std::process::exit(1);
    }
    let yaml = fs_err::read_to_string(&args[1]).expect("Failed to read YAML file");
    let offering = Offering::new_from_str(&yaml, "yaml").expect("Failed to parse YAML");
    if offering.matches_search(&args[2]) {
        eprintln!("Profile matches!");
        println!("{}", offering);
    } else {
        eprintln!("Profile does not match!");
        std::process::exit(1);
    }
}
