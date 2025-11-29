use serde::{Deserialize, Serialize};

/// Stripe-supported currencies (lowercase as required by Stripe API)
/// Source: https://stripe.com/docs/currencies
pub const STRIPE_SUPPORTED_CURRENCIES: &[&str] = &[
    "usd", "aed", "afn", "all", "amd", "ang", "aoa", "ars", "aud", "awg", "azn", "bam", "bbd",
    "bdt", "bgn", "bif", "bmd", "bnd", "bob", "brl", "bsd", "bwp", "byn", "bzd", "cad", "cdf",
    "chf", "clp", "cny", "cop", "crc", "cve", "czk", "djf", "dkk", "dop", "dzd", "egp", "etb",
    "eur", "fjd", "fkp", "gbp", "gel", "gip", "gmd", "gnf", "gtq", "gyd", "hkd", "hnl", "hrk",
    "htg", "huf", "idr", "ils", "inr", "isk", "jmd", "jpy", "kes", "kgs", "khr", "kmf", "krw",
    "kyd", "kzt", "lak", "lbp", "lkr", "lrd", "lsl", "mad", "mdl", "mga", "mkd", "mmk", "mnt",
    "mop", "mur", "mvr", "mwk", "mxn", "myr", "mzn", "nad", "ngn", "nio", "nok", "npr", "nzd",
    "pab", "pen", "pgk", "php", "pkr", "pln", "pyg", "qar", "ron", "rsd", "rub", "rwf", "sar",
    "sbd", "scr", "sek", "sgd", "shp", "sle", "sos", "srd", "std", "szl", "thb", "tjs", "top",
    "try", "ttd", "twd", "tzs", "uah", "ugx", "uyu", "uzs", "vnd", "vuv", "wst", "xaf", "xcd",
    "xof", "xpf", "yer", "zar", "zmw",
];

/// Payment method for contracts
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    #[serde(rename = "dct")]
    DCT,
    Stripe,
}

impl PaymentMethod {
    pub fn is_dct(&self) -> bool {
        matches!(self, PaymentMethod::DCT)
    }

    pub fn is_stripe(&self) -> bool {
        matches!(self, PaymentMethod::Stripe)
    }
}

impl std::fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentMethod::DCT => write!(f, "dct"),
            PaymentMethod::Stripe => write!(f, "stripe"),
        }
    }
}

impl std::str::FromStr for PaymentMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dct" => Ok(PaymentMethod::DCT),
            "stripe" => Ok(PaymentMethod::Stripe),
            _ => Err(format!("Invalid payment method: {}", s)),
        }
    }
}

/// Check if a currency is supported by Stripe
pub fn is_stripe_supported_currency(currency: &str) -> bool {
    STRIPE_SUPPORTED_CURRENCIES.contains(&currency.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_method_is_dct() {
        assert!(PaymentMethod::DCT.is_dct());
        assert!(!PaymentMethod::Stripe.is_dct());
    }

    #[test]
    fn test_payment_method_is_stripe() {
        assert!(PaymentMethod::Stripe.is_stripe());
        assert!(!PaymentMethod::DCT.is_stripe());
    }

    #[test]
    fn test_payment_method_from_str_valid() {
        assert_eq!("dct".parse::<PaymentMethod>().unwrap(), PaymentMethod::DCT);
        assert_eq!("DCT".parse::<PaymentMethod>().unwrap(), PaymentMethod::DCT);
        assert_eq!(
            "stripe".parse::<PaymentMethod>().unwrap(),
            PaymentMethod::Stripe
        );
        assert_eq!(
            "Stripe".parse::<PaymentMethod>().unwrap(),
            PaymentMethod::Stripe
        );
        assert_eq!(
            "STRIPE".parse::<PaymentMethod>().unwrap(),
            PaymentMethod::Stripe
        );
    }

    #[test]
    fn test_payment_method_from_str_invalid() {
        assert!("paypal".parse::<PaymentMethod>().is_err());
        assert!("bitcoin".parse::<PaymentMethod>().is_err());
        assert!("".parse::<PaymentMethod>().is_err());
    }

    #[test]
    fn test_payment_method_display() {
        assert_eq!(PaymentMethod::DCT.to_string(), "dct");
        assert_eq!(PaymentMethod::Stripe.to_string(), "stripe");
    }

    #[test]
    fn test_payment_method_serialize() {
        let dct = PaymentMethod::DCT;
        let json = serde_json::to_string(&dct).unwrap();
        assert_eq!(json, r#""dct""#);

        let stripe = PaymentMethod::Stripe;
        let json = serde_json::to_string(&stripe).unwrap();
        assert_eq!(json, r#""stripe""#);
    }

    #[test]
    fn test_payment_method_deserialize() {
        let dct: PaymentMethod = serde_json::from_str(r#""dct""#).unwrap();
        assert_eq!(dct, PaymentMethod::DCT);

        let stripe: PaymentMethod = serde_json::from_str(r#""stripe""#).unwrap();
        assert_eq!(stripe, PaymentMethod::Stripe);
    }

    #[test]
    fn test_payment_method_deserialize_invalid() {
        let result: Result<PaymentMethod, _> = serde_json::from_str(r#""paypal""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_stripe_supported_currency_valid() {
        assert!(is_stripe_supported_currency("usd"));
        assert!(is_stripe_supported_currency("USD"));
        assert!(is_stripe_supported_currency("eur"));
        assert!(is_stripe_supported_currency("EUR"));
        assert!(is_stripe_supported_currency("gbp"));
        assert!(is_stripe_supported_currency("jpy"));
        assert!(is_stripe_supported_currency("cad"));
    }

    #[test]
    fn test_is_stripe_supported_currency_invalid() {
        assert!(!is_stripe_supported_currency("btc"));
        assert!(!is_stripe_supported_currency("eth"));
        assert!(!is_stripe_supported_currency("invalid"));
        assert!(!is_stripe_supported_currency(""));
        assert!(!is_stripe_supported_currency("dct"));
    }
}
