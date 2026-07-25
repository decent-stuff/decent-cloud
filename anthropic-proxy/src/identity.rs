//! Per-identity resolution (spec I.1: beta serves ONE identity per host).

use std::sync::Arc;

/// A stable reference to the identity a proxied request belongs to.
/// `id` corresponds to `agent_identities.id` (UUID) in the central API DB.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct IdentityRef {
    pub id: String,
}

impl IdentityRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Resolves which identity a proxied request belongs to.
///
/// Beta impl: a single fixed identity configured at proxy startup (dc-agent passes it
/// when spawning the process). Future multi-tenant extension: implement this against a
/// per-container bearer-token table — but do NOT build that path now (YAGNI for beta).
#[async_trait::async_trait]
pub trait IdentityResolver: Send + Sync {
    async fn resolve(&self) -> anyhow::Result<IdentityRef>;
}

/// Single-identity resolver: every request belongs to the configured identity.
#[derive(Debug, Clone)]
pub struct SingleIdentityResolver {
    identity: IdentityRef,
}

impl SingleIdentityResolver {
    pub fn new(identity: IdentityRef) -> Self {
        Self { identity }
    }
}

#[async_trait::async_trait]
impl IdentityResolver for SingleIdentityResolver {
    async fn resolve(&self) -> anyhow::Result<IdentityRef> {
        Ok(self.identity.clone())
    }
}

pub type SharedResolver = Arc<dyn IdentityResolver>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn single_resolver_returns_configured_identity() {
        let r = SingleIdentityResolver::new(IdentityRef::new("id-7"));
        let got = r.resolve().await.expect("resolve ok");
        assert_eq!(got, IdentityRef::new("id-7"));
    }

    #[tokio::test]
    async fn single_resolver_is_stable_across_calls() {
        let r = SingleIdentityResolver::new(IdentityRef::new("stable"));
        let a = r.resolve().await.unwrap();
        let b = r.resolve().await.unwrap();
        assert_eq!(a, b);
    }
}
