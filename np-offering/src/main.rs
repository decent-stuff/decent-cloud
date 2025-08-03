use np_offering::ProviderOfferings;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <CSV file> [search_field] [search_value]",
            args[0]
        );
        eprintln!("Example: {} offerings.csv offer_name \"Intel\"", args[0]);
        std::process::exit(1);
    }

    let offerings = match ProviderOfferings::new_from_file(&[], &args[1]) {
        Ok(offerings) => offerings,
        Err(e) => {
            eprintln!("Failed to parse CSV file: {}", e);
            std::process::exit(1);
        }
    };

    println!(
        "Loaded {} offerings from CSV",
        offerings.server_offerings.len()
    );

    // If search parameters provided, filter results
    if args.len() >= 4 {
        let search_field = &args[2];
        let search_value = &args[3];

        let filtered = match search_field.as_str() {
            "offer_name" => offerings.find_by_name(search_value),
            "product_type" => match search_value.to_lowercase().as_str() {
                "vps" => offerings.find_by_product_type(&np_offering::ProductType::VPS),
                "dedicated" => offerings.find_by_product_type(&np_offering::ProductType::Dedicated),
                "cloud" => offerings.find_by_product_type(&np_offering::ProductType::Cloud),
                "managed" => offerings.find_by_product_type(&np_offering::ProductType::Managed),
                _ => {
                    eprintln!("Unknown product type: {}", search_value);
                    std::process::exit(1);
                }
            },
            "country" => offerings.find_by_country(search_value),
            "gpu" => {
                if search_value.to_lowercase() == "true" {
                    offerings.find_with_gpu()
                } else {
                    offerings.filter(|offering| offering.gpu_name.is_none())
                }
            }
            _ => {
                eprintln!("Unknown search field: {}", search_field);
                eprintln!("Available fields: offer_name, product_type, country, gpu");
                std::process::exit(1);
            }
        };

        println!("Found {} matching offerings:", filtered.len());
        for offering in filtered {
            println!(
                "- {} ({}): €{}/month",
                offering.offer_name, offering.unique_internal_identifier, offering.monthly_price
            );
        }
    } else {
        // Just list all offerings
        for offering in &offerings.server_offerings {
            println!(
                "- {} ({}): €{}/month",
                offering.offer_name, offering.unique_internal_identifier, offering.monthly_price
            );
        }
    }
}
