use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_main_help_output() {
    // Test that the main function shows help when no arguments are provided
    let args = vec![];

    // Capture stderr since help is printed to stderr
    let output = capture_main_output(args);
    assert!(output.contains("Usage:"));
    assert!(output.contains("CSV file"));
}

#[test]
fn test_main_file_not_found() {
    // Test that the main function handles missing files gracefully
    let args = vec![String::from("nonexistent_file.csv")];

    let output = capture_main_output(args);
    assert!(output.contains("Failed to parse CSV file"));
}

#[test]
fn test_main_with_valid_csv() {
    // Create a temporary CSV file
    let csv_content = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Test Server,A test server,TEST001,https://test.com,USD,29.99,0,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.4 GHz,Intel Xeon,ECC,DDR4,8 GB,0,0,1,100 GB,Standard,1 Gbit,1024,US,New York,"40.7128,-74.0060",KVM over IP,Ubuntu,cPanel,,Credit Card"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    fs::write(path, csv_content).unwrap();

    let args = vec![
        path.to_str().unwrap().to_string(),
    ];

    let output = capture_main_output(args);
    assert!(output.contains("Loaded 1 offerings from CSV"));
    assert!(output.contains("Test Server"));
}

#[test]
fn test_main_with_search() {
    // Create a temporary CSV file
    let csv_content = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Intel Server,Intel server,INTEL001,https://test.com,USD,99.99,0,Visible,VPS,KVM,Monthly,In stock,Intel,1,4,2.4 GHz,Intel Xeon,ECC,DDR4,16 GB,0,0,1,240 GB,Standard,1 Gbit,5120,US,New York,"40.7128,-74.0060",KVM over IP,Ubuntu,cPanel,NVIDIA GTX 1080,Credit Card
AMD Server,AMD server,AMD001,https://test.com,USD,49.99,0,Visible,VPS,KVM,Monthly,In stock,AMD,1,2,2.0 GHz,AMD Opteron,ECC,DDR4,8 GB,0,0,1,100 GB,Standard,1 Gbit,1024,US,Dallas,"32.7767,-96.7970",KVM over IP,Ubuntu,cPanel,,PayPal"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    fs::write(path, csv_content).unwrap();

    // Test search by name
    let args = vec![
        path.to_str().unwrap().to_string(),
        String::from("offer_name"),
        String::from("Intel"),
    ];

    let output = capture_main_output(args);
    println!("Output: {}", output);
    assert!(output.contains("Found 1 matching offerings:"));
    assert!(output.contains("Intel Server"));

    // Test search by product type
    let args = vec![
        path.to_str().unwrap().to_string(),
        String::from("product_type"),
        String::from("vps"),
    ];

    let output = capture_main_output(args);
    assert!(output.contains("Found 2 matching offerings:"));
}

#[test]
fn test_main_invalid_search_field() {
    // Create a temporary CSV file
    let csv_content = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Test Server,A test server,TEST001,https://test.com,USD,29.99,0,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.4 GHz,Intel Xeon,ECC,DDR4,8 GB,0,0,1,100 GB,Standard,1 Gbit,1024,US,New York,"40.7128,-74.0060",KVM over IP,Ubuntu,cPanel,,Credit Card"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    fs::write(path, csv_content).unwrap();

    let args = vec![
        path.to_str().unwrap().to_string(),
        String::from("invalid_field"),
        String::from("test"),
    ];

    let output = capture_main_output(args);
    assert!(output.contains("Unknown search field"));
}

#[test]
fn test_main_invalid_product_type() {
    // Create a temporary CSV file
    let csv_content = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Test Server,A test server,TEST001,https://test.com,USD,29.99,0,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.4 GHz,Intel Xeon,ECC,DDR4,8 GB,0,0,1,100 GB,Standard,1 Gbit,1024,US,New York,"40.7128,-74.0060",KVM over IP,Ubuntu,cPanel,,Credit Card"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    fs::write(path, csv_content).unwrap();

    let args = vec![
        path.to_str().unwrap().to_string(),
        String::from("product_type"),
        String::from("invalid_type"),
    ];

    let output = capture_main_output(args);
    assert!(output.contains("Unknown product type"));
}

// Helper function to capture the output of the main function
fn capture_main_output(args: Vec<String>) -> String {
    // Build the command to run the main binary
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(&["run", "--bin", "np-offering", "--"]).args(&args);

    // Run the command and capture output
    let output = cmd.output().unwrap();

    // Combine stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    format!("{}{}", stdout, stderr)
}
