use serde::{Deserialize, Serialize};

/// Contract status state machine
///
/// Valid state transitions:
/// - Requested -> Pending (auto-accept), Accepted (manual accept), Rejected, Cancelled
/// - Pending -> Accepted, Rejected, Cancelled
/// - Accepted -> Provisioning, Rejected, Cancelled
/// - Provisioning -> Provisioned, Cancelled (failed provisioning)
/// - Provisioned -> Active, Cancelled
/// - Active -> Cancelled, Expired
/// - Rejected, Cancelled, Expired are terminal states
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractStatus {
    /// Initial state - user has requested the contract
    Requested,
    /// Provider auto-accepted, waiting for provider action
    Pending,
    /// Provider has manually accepted the request
    Accepted,
    /// Provider is setting up the VM/service
    Provisioning,
    /// VM/service is ready, credentials available
    Provisioned,
    /// Contract is fully operational
    Active,
    /// Provider rejected the request (terminal)
    Rejected,
    /// User or provider cancelled (terminal)
    Cancelled,
    /// Contract duration ended (terminal)
    Expired,
}

impl ContractStatus {
    /// Check if this status can transition to the target status
    pub fn can_transition_to(&self, target: ContractStatus) -> bool {
        use ContractStatus::*;
        match (self, target) {
            // From Requested
            (Requested, Pending) => true,
            (Requested, Accepted) => true,
            (Requested, Rejected) => true,
            (Requested, Cancelled) => true,
            // From Pending
            (Pending, Accepted) => true,
            (Pending, Rejected) => true,
            (Pending, Cancelled) => true,
            // From Accepted
            (Accepted, Provisioning) => true,
            (Accepted, Rejected) => true,
            (Accepted, Cancelled) => true,
            // From Provisioning
            (Provisioning, Provisioned) => true,
            (Provisioning, Cancelled) => true, // Failed provisioning
            // From Provisioned
            (Provisioned, Active) => true,
            (Provisioned, Cancelled) => true,
            // From Active
            (Active, Cancelled) => true,
            (Active, Expired) => true,
            // Terminal states cannot transition
            (Rejected | Cancelled | Expired, _) => false,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ContractStatus::Rejected | ContractStatus::Cancelled | ContractStatus::Expired
        )
    }

    /// Check if this status indicates the contract can be cancelled
    pub fn is_cancellable(&self) -> bool {
        matches!(
            self,
            ContractStatus::Requested
                | ContractStatus::Pending
                | ContractStatus::Accepted
                | ContractStatus::Provisioning
                | ContractStatus::Provisioned
                | ContractStatus::Active
        )
    }

    /// Check if this status indicates the contract is operational (user has access)
    pub fn is_operational(&self) -> bool {
        matches!(self, ContractStatus::Provisioned | ContractStatus::Active)
    }

    /// Returns all valid transitions from this status
    pub fn valid_transitions(&self) -> &'static [ContractStatus] {
        use ContractStatus::*;
        match self {
            Requested => &[Pending, Accepted, Rejected, Cancelled],
            Pending => &[Accepted, Rejected, Cancelled],
            Accepted => &[Provisioning, Rejected, Cancelled],
            Provisioning => &[Provisioned, Cancelled],
            Provisioned => &[Active, Cancelled],
            Active => &[Cancelled, Expired],
            Rejected | Cancelled | Expired => &[],
        }
    }
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Requested => write!(f, "requested"),
            ContractStatus::Pending => write!(f, "pending"),
            ContractStatus::Accepted => write!(f, "accepted"),
            ContractStatus::Provisioning => write!(f, "provisioning"),
            ContractStatus::Provisioned => write!(f, "provisioned"),
            ContractStatus::Active => write!(f, "active"),
            ContractStatus::Rejected => write!(f, "rejected"),
            ContractStatus::Cancelled => write!(f, "cancelled"),
            ContractStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for ContractStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "requested" => Ok(ContractStatus::Requested),
            "pending" => Ok(ContractStatus::Pending),
            "accepted" => Ok(ContractStatus::Accepted),
            "provisioning" => Ok(ContractStatus::Provisioning),
            "provisioned" => Ok(ContractStatus::Provisioned),
            "active" => Ok(ContractStatus::Active),
            "rejected" => Ok(ContractStatus::Rejected),
            "cancelled" | "canceled" => Ok(ContractStatus::Cancelled),
            "expired" => Ok(ContractStatus::Expired),
            _ => Err(format!(
                "Invalid contract status '{}'. Valid statuses: requested, pending, accepted, provisioning, provisioned, active, rejected, cancelled, expired",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid() {
        assert_eq!(
            "requested".parse::<ContractStatus>().unwrap(),
            ContractStatus::Requested
        );
        assert_eq!(
            "REQUESTED".parse::<ContractStatus>().unwrap(),
            ContractStatus::Requested
        );
        assert_eq!(
            "pending".parse::<ContractStatus>().unwrap(),
            ContractStatus::Pending
        );
        assert_eq!(
            "accepted".parse::<ContractStatus>().unwrap(),
            ContractStatus::Accepted
        );
        assert_eq!(
            "provisioning".parse::<ContractStatus>().unwrap(),
            ContractStatus::Provisioning
        );
        assert_eq!(
            "provisioned".parse::<ContractStatus>().unwrap(),
            ContractStatus::Provisioned
        );
        assert_eq!(
            "active".parse::<ContractStatus>().unwrap(),
            ContractStatus::Active
        );
        assert_eq!(
            "rejected".parse::<ContractStatus>().unwrap(),
            ContractStatus::Rejected
        );
        assert_eq!(
            "cancelled".parse::<ContractStatus>().unwrap(),
            ContractStatus::Cancelled
        );
        // Accept "canceled" (American spelling)
        assert_eq!(
            "canceled".parse::<ContractStatus>().unwrap(),
            ContractStatus::Cancelled
        );
        assert_eq!(
            "expired".parse::<ContractStatus>().unwrap(),
            ContractStatus::Expired
        );
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("invalid".parse::<ContractStatus>().is_err());
        assert!("".parse::<ContractStatus>().is_err());
        assert!("completed".parse::<ContractStatus>().is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(ContractStatus::Requested.to_string(), "requested");
        assert_eq!(ContractStatus::Pending.to_string(), "pending");
        assert_eq!(ContractStatus::Accepted.to_string(), "accepted");
        assert_eq!(ContractStatus::Provisioning.to_string(), "provisioning");
        assert_eq!(ContractStatus::Provisioned.to_string(), "provisioned");
        assert_eq!(ContractStatus::Active.to_string(), "active");
        assert_eq!(ContractStatus::Rejected.to_string(), "rejected");
        assert_eq!(ContractStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(ContractStatus::Expired.to_string(), "expired");
    }

    #[test]
    fn test_valid_transitions_requested() {
        let status = ContractStatus::Requested;
        assert!(status.can_transition_to(ContractStatus::Pending));
        assert!(status.can_transition_to(ContractStatus::Accepted));
        assert!(status.can_transition_to(ContractStatus::Rejected));
        assert!(status.can_transition_to(ContractStatus::Cancelled));
        assert!(!status.can_transition_to(ContractStatus::Provisioning));
        assert!(!status.can_transition_to(ContractStatus::Provisioned));
        assert!(!status.can_transition_to(ContractStatus::Active));
    }

    #[test]
    fn test_valid_transitions_pending() {
        let status = ContractStatus::Pending;
        assert!(status.can_transition_to(ContractStatus::Accepted));
        assert!(status.can_transition_to(ContractStatus::Rejected));
        assert!(status.can_transition_to(ContractStatus::Cancelled));
        assert!(!status.can_transition_to(ContractStatus::Requested));
        assert!(!status.can_transition_to(ContractStatus::Provisioning));
    }

    #[test]
    fn test_valid_transitions_accepted() {
        let status = ContractStatus::Accepted;
        assert!(status.can_transition_to(ContractStatus::Provisioning));
        assert!(status.can_transition_to(ContractStatus::Rejected));
        assert!(status.can_transition_to(ContractStatus::Cancelled));
        assert!(!status.can_transition_to(ContractStatus::Requested));
        assert!(!status.can_transition_to(ContractStatus::Pending));
        assert!(!status.can_transition_to(ContractStatus::Provisioned));
    }

    #[test]
    fn test_valid_transitions_provisioning() {
        let status = ContractStatus::Provisioning;
        assert!(status.can_transition_to(ContractStatus::Provisioned));
        assert!(status.can_transition_to(ContractStatus::Cancelled));
        assert!(!status.can_transition_to(ContractStatus::Requested));
        assert!(!status.can_transition_to(ContractStatus::Accepted));
        assert!(!status.can_transition_to(ContractStatus::Active));
    }

    #[test]
    fn test_valid_transitions_provisioned() {
        let status = ContractStatus::Provisioned;
        assert!(status.can_transition_to(ContractStatus::Active));
        assert!(status.can_transition_to(ContractStatus::Cancelled));
        assert!(!status.can_transition_to(ContractStatus::Requested));
        assert!(!status.can_transition_to(ContractStatus::Provisioning));
    }

    #[test]
    fn test_valid_transitions_active() {
        let status = ContractStatus::Active;
        assert!(status.can_transition_to(ContractStatus::Cancelled));
        assert!(status.can_transition_to(ContractStatus::Expired));
        assert!(!status.can_transition_to(ContractStatus::Requested));
        assert!(!status.can_transition_to(ContractStatus::Provisioned));
    }

    #[test]
    fn test_terminal_states_cannot_transition() {
        for terminal in [
            ContractStatus::Rejected,
            ContractStatus::Cancelled,
            ContractStatus::Expired,
        ] {
            for target in [
                ContractStatus::Requested,
                ContractStatus::Pending,
                ContractStatus::Accepted,
                ContractStatus::Provisioning,
                ContractStatus::Provisioned,
                ContractStatus::Active,
                ContractStatus::Rejected,
                ContractStatus::Cancelled,
                ContractStatus::Expired,
            ] {
                assert!(
                    !terminal.can_transition_to(target),
                    "{:?} should not transition to {:?}",
                    terminal,
                    target
                );
            }
        }
    }

    #[test]
    fn test_is_terminal() {
        assert!(!ContractStatus::Requested.is_terminal());
        assert!(!ContractStatus::Pending.is_terminal());
        assert!(!ContractStatus::Accepted.is_terminal());
        assert!(!ContractStatus::Provisioning.is_terminal());
        assert!(!ContractStatus::Provisioned.is_terminal());
        assert!(!ContractStatus::Active.is_terminal());
        assert!(ContractStatus::Rejected.is_terminal());
        assert!(ContractStatus::Cancelled.is_terminal());
        assert!(ContractStatus::Expired.is_terminal());
    }

    #[test]
    fn test_is_cancellable() {
        assert!(ContractStatus::Requested.is_cancellable());
        assert!(ContractStatus::Pending.is_cancellable());
        assert!(ContractStatus::Accepted.is_cancellable());
        assert!(ContractStatus::Provisioning.is_cancellable());
        assert!(ContractStatus::Provisioned.is_cancellable());
        assert!(ContractStatus::Active.is_cancellable());
        assert!(!ContractStatus::Rejected.is_cancellable());
        assert!(!ContractStatus::Cancelled.is_cancellable());
        assert!(!ContractStatus::Expired.is_cancellable());
    }

    #[test]
    fn test_is_operational() {
        assert!(!ContractStatus::Requested.is_operational());
        assert!(!ContractStatus::Pending.is_operational());
        assert!(!ContractStatus::Accepted.is_operational());
        assert!(!ContractStatus::Provisioning.is_operational());
        assert!(ContractStatus::Provisioned.is_operational());
        assert!(ContractStatus::Active.is_operational());
        assert!(!ContractStatus::Rejected.is_operational());
        assert!(!ContractStatus::Cancelled.is_operational());
        assert!(!ContractStatus::Expired.is_operational());
    }

    #[test]
    fn test_valid_transitions_list() {
        assert_eq!(
            ContractStatus::Requested.valid_transitions(),
            &[
                ContractStatus::Pending,
                ContractStatus::Accepted,
                ContractStatus::Rejected,
                ContractStatus::Cancelled
            ]
        );
        assert_eq!(ContractStatus::Rejected.valid_transitions(), &[]);
        assert_eq!(ContractStatus::Cancelled.valid_transitions(), &[]);
        assert_eq!(ContractStatus::Expired.valid_transitions(), &[]);
    }

    #[test]
    fn test_serialize() {
        let status = ContractStatus::Accepted;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""accepted""#);
    }

    #[test]
    fn test_deserialize() {
        let status: ContractStatus = serde_json::from_str(r#""provisioned""#).unwrap();
        assert_eq!(status, ContractStatus::Provisioned);
    }
}
