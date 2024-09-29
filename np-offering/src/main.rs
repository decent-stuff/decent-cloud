use np_offering::Offering;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <YAML file> <Search string>", args[0]);
        std::process::exit(1);
    }
    let yaml = std::fs::read_to_string(&args[1]).expect("Failed to read YAML file");
    if Offering::search(&yaml, &args[2]) {
        eprintln!("Profile matches!");
        match Offering::parse(&yaml) {
            Ok(offering) => println!("{}", offering),
            Err(e) => {
                eprintln!("Error parsing offering: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Profile does not match!");
        std::process::exit(1);
    }
}
