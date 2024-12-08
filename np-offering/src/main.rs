use np_json_search::value_matches_with_parents;
use np_offering::Offering;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <YAML/JSON file> <Search string>", args[0]);
        std::process::exit(1);
    }

    let offering = Offering::new_from_file(&args[1]).expect("Failed to parse offering");
    let matching_instances =
        value_matches_with_parents(offering.json_value(), "instance_types.id", &args[2]);
    if matching_instances.is_empty() {
        eprintln!("Offering does not match!");
        std::process::exit(1);
    } else {
        for instance in matching_instances {
            if instance.is_empty() {
                eprintln!("Offering matches, but no particular instance matched!");
                println!("{}", offering.as_json_string().unwrap());
            } else {
                eprintln!("Matching instance id: {}", instance);
            }
        }
    }
}
