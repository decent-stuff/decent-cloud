use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Currency {
    EUR,
    USD,
    USDT,
    BTC,
    ETH,
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::EUR => write!(f, "EUR"),
            Currency::USD => write!(f, "USD"),
            Currency::USDT => write!(f, "USDT"),
            Currency::BTC => write!(f, "BTC"),
            Currency::ETH => write!(f, "ETH"),
        }
    }
}

impl FromStr for Currency {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "EUR" => Ok(Currency::EUR),
            "USD" => Ok(Currency::USD),
            "USDT" => Ok(Currency::USDT),
            "BTC" => Ok(Currency::BTC),
            "ETH" => Ok(Currency::ETH),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Visibility {
    Visible,
    Invisible,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Visible => write!(f, "Visible"),
            Visibility::Invisible => write!(f, "Invisible"),
        }
    }
}

impl FromStr for Visibility {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "visible" => Ok(Visibility::Visible),
            "invisible" => Ok(Visibility::Invisible),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProductType {
    VPS,
    Dedicated,
    Cloud,
    Managed,
}

impl std::fmt::Display for ProductType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductType::VPS => write!(f, "VPS"),
            ProductType::Dedicated => write!(f, "Dedicated"),
            ProductType::Cloud => write!(f, "Cloud"),
            ProductType::Managed => write!(f, "Managed"),
        }
    }
}

impl FromStr for ProductType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "vps" => Ok(ProductType::VPS),
            "dedicated" => Ok(ProductType::Dedicated),
            "cloud" => Ok(ProductType::Cloud),
            "managed" => Ok(ProductType::Managed),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VirtualizationType {
    KVM,
    VMware,
    Xen,
    HyperV,
    None,
}

impl std::fmt::Display for VirtualizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VirtualizationType::KVM => write!(f, "KVM"),
            VirtualizationType::VMware => write!(f, "VMware"),
            VirtualizationType::Xen => write!(f, "Xen"),
            VirtualizationType::HyperV => write!(f, "Hyper-V"),
            VirtualizationType::None => write!(f, "None"),
        }
    }
}

impl FromStr for VirtualizationType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "kvm" => Ok(VirtualizationType::KVM),
            "vmware" => Ok(VirtualizationType::VMware),
            "xen" => Ok(VirtualizationType::Xen),
            "hyper-v" | "hyperv" => Ok(VirtualizationType::HyperV),
            "none" => Ok(VirtualizationType::None),
            "" => Ok(VirtualizationType::None),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BillingInterval {
    Hourly,
    Daily,
    Monthly,
    Yearly,
}

impl std::fmt::Display for BillingInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BillingInterval::Hourly => write!(f, "Hourly"),
            BillingInterval::Daily => write!(f, "Daily"),
            BillingInterval::Monthly => write!(f, "Monthly"),
            BillingInterval::Yearly => write!(f, "Yearly"),
        }
    }
}

impl FromStr for BillingInterval {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hourly" | "hour" => Ok(BillingInterval::Hourly),
            "daily" | "day" => Ok(BillingInterval::Daily),
            "monthly" | "month" => Ok(BillingInterval::Monthly),
            "yearly" | "year" => Ok(BillingInterval::Yearly),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StockStatus {
    InStock,
    OutOfStock,
    Limited,
}

impl std::fmt::Display for StockStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StockStatus::InStock => write!(f, "In stock"),
            StockStatus::OutOfStock => write!(f, "Out of stock"),
            StockStatus::Limited => write!(f, "Limited"),
        }
    }
}

impl FromStr for StockStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "in stock" | "in-stock" => Ok(StockStatus::InStock),
            "out of stock" | "out-of-stock" => Ok(StockStatus::OutOfStock),
            "limited" => Ok(StockStatus::Limited),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ErrorCorrection {
    ECC,
    ECCRegistered,
    NonECC,
}

impl std::fmt::Display for ErrorCorrection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCorrection::ECC => write!(f, "ECC"),
            ErrorCorrection::ECCRegistered => write!(f, "ECC Registered"),
            ErrorCorrection::NonECC => write!(f, "non-ECC"),
        }
    }
}

impl FromStr for ErrorCorrection {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ecc" => Ok(ErrorCorrection::ECC),
            "ecc registered" | "ecc-registered" | "ecc-reg" | "eccreg" => {
                Ok(ErrorCorrection::ECCRegistered)
            }
            "non-ecc" | "nonecc" | "non ecc" => Ok(ErrorCorrection::NonECC),
            _ => Err(()),
        }
    }
}