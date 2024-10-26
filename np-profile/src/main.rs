use np_profile::Profile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <YAML/JSON file> <Search string>", args[0]);
        std::process::exit(1);
    }

    let profile = Profile::new_from_file(&args[1]).expect("Failed to parse profile");
    if profile.matches_search(&args[2]) {
        eprintln!("Profile matches!");
        println!("{}", profile)
    } else {
        eprintln!("Profile does not match!");
        std::process::exit(1);
    }
}
