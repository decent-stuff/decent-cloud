use np_profile::Profile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <YAML file> <Search string>", args[0]);
        std::process::exit(1);
    }
    let yaml = fs_err::read_to_string(&args[1]).expect("Failed to read YAML file");
    let profile = Profile::new_from_str(&yaml, "yaml").expect("Failed to parse YAML");
    if profile.matches_search(&args[2]) {
        eprintln!("Profile matches!");
        println!("{}", profile)
    } else {
        eprintln!("Profile does not match!");
        std::process::exit(1);
    }
}
