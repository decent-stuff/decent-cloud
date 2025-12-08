use anyhow::{Context, Result};
use serde::Deserialize;

const VIES_SOAP_URL: &str = "https://ec.europa.eu/taxation_customs/vies/services/checkVatService";

/// VIES VAT validation response
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ViesResponse {
    pub valid: bool,
    pub name: Option<String>,
    pub address: Option<String>,
}

/// Validates a VAT ID using the EU VIES SOAP API
///
/// # Arguments
/// * `country_code` - Two-letter country code (e.g., "DE", "FR")
/// * `vat_number` - VAT number without country prefix
///
/// # Returns
/// ViesResponse with validation status and optional company name/address
pub async fn validate_vat_id(country_code: &str, vat_number: &str) -> Result<ViesResponse> {
    let soap_request = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
                  xmlns:urn="urn:ec.europa.eu:taxud:vies:services:checkVat:types">
   <soapenv:Body>
      <urn:checkVat>
         <urn:countryCode>{}</urn:countryCode>
         <urn:vatNumber>{}</urn:vatNumber>
      </urn:checkVat>
   </soapenv:Body>
</soapenv:Envelope>"#,
        country_code, vat_number
    );

    let client = reqwest::Client::new();
    let response = client
        .post(VIES_SOAP_URL)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", "")
        .body(soap_request)
        .send()
        .await
        .context("Failed to send VIES SOAP request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("VIES API error ({}): {}", status, body));
    }

    let body = response
        .text()
        .await
        .context("Failed to read VIES response body")?;
    parse_vies_response(&body)
}

/// Parses VIES SOAP XML response
fn parse_vies_response(xml: &str) -> Result<ViesResponse> {
    // Extract valid field
    let valid = xml.contains("<valid>true</valid>");

    // Extract name (optional)
    let name = extract_xml_value(xml, "name");

    // Extract address (optional)
    let address = extract_xml_value(xml, "address");

    Ok(ViesResponse {
        valid,
        name,
        address,
    })
}

/// Extracts value from XML tag
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    if let Some(start) = xml.find(&start_tag) {
        if let Some(end) = xml[start..].find(&end_tag) {
            let value = &xml[start + start_tag.len()..start + end];
            if !value.trim().is_empty() && value != "---" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vies_response_valid() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
   <soap:Body>
      <checkVatResponse xmlns="urn:ec.europa.eu:taxud:vies:services:checkVat:types">
         <countryCode>DE</countryCode>
         <vatNumber>123456789</vatNumber>
         <requestDate>2025-12-08+01:00</requestDate>
         <valid>true</valid>
         <name>Example GmbH</name>
         <address>Musterstrasse 1, 12345 Berlin</address>
      </checkVatResponse>
   </soap:Body>
</soap:Envelope>"#;

        let result = parse_vies_response(xml).unwrap();
        assert_eq!(result.valid, true);
        assert_eq!(result.name, Some("Example GmbH".to_string()));
        assert_eq!(
            result.address,
            Some("Musterstrasse 1, 12345 Berlin".to_string())
        );
    }

    #[test]
    fn test_parse_vies_response_invalid() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
   <soap:Body>
      <checkVatResponse xmlns="urn:ec.europa.eu:taxud:vies:services:checkVat:types">
         <countryCode>DE</countryCode>
         <vatNumber>999999999</vatNumber>
         <requestDate>2025-12-08+01:00</requestDate>
         <valid>false</valid>
         <name>---</name>
         <address>---</address>
      </checkVatResponse>
   </soap:Body>
</soap:Envelope>"#;

        let result = parse_vies_response(xml).unwrap();
        assert_eq!(result.valid, false);
        assert_eq!(result.name, None);
        assert_eq!(result.address, None);
    }

    #[test]
    fn test_parse_vies_response_empty_fields() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
   <soap:Body>
      <checkVatResponse xmlns="urn:ec.europa.eu:taxud:vies:services:checkVat:types">
         <countryCode>FR</countryCode>
         <vatNumber>12345678901</vatNumber>
         <requestDate>2025-12-08+01:00</requestDate>
         <valid>true</valid>
         <name></name>
         <address></address>
      </checkVatResponse>
   </soap:Body>
</soap:Envelope>"#;

        let result = parse_vies_response(xml).unwrap();
        assert_eq!(result.valid, true);
        assert_eq!(result.name, None);
        assert_eq!(result.address, None);
    }

    #[test]
    fn test_extract_xml_value() {
        let xml = "<root><name>Test Company</name><other>value</other></root>";
        assert_eq!(
            extract_xml_value(xml, "name"),
            Some("Test Company".to_string())
        );
        assert_eq!(extract_xml_value(xml, "other"), Some("value".to_string()));
        assert_eq!(extract_xml_value(xml, "missing"), None);
    }

    #[test]
    fn test_extract_xml_value_empty() {
        let xml = "<root><name></name></root>";
        assert_eq!(extract_xml_value(xml, "name"), None);
    }

    #[test]
    fn test_extract_xml_value_dashes() {
        let xml = "<root><name>---</name></root>";
        assert_eq!(extract_xml_value(xml, "name"), None);
    }
}
