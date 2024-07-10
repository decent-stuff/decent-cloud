use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct NodeProvider {
    pub name: String,
    pub pubkey: Vec<u8>,
    pub reputation: i32,
}

impl NodeProvider {
    pub fn new(
        name: String,
        pubkey: Vec<u8>,
        reputation: i32,
    ) -> Result<NodeProvider, &'static str> {
        if name.is_empty() || pubkey.is_empty() || reputation < 0 {
            return Err("Invalid input");
        }

        Ok(NodeProvider {
            name,
            pubkey,
            reputation,
        })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_pubkey(&self) -> &[u8] {
        &self.pubkey
    }

    pub fn get_reputation(&self) -> i32 {
        self.reputation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_node_provider() {
        let node_provider = NodeProvider::new("Node1".to_string(), vec![1, 2, 3], 10);
        assert!(node_provider.is_ok());
    }

    #[test]
    fn test_invalid_name() {
        let node_provider = NodeProvider::new("".to_string(), vec![1, 2, 3], 10);
        assert!(node_provider.is_err());
    }

    #[test]
    fn test_invalid_pubkey() {
        let node_provider = NodeProvider::new("Node1".to_string(), vec![], 10);
        assert!(node_provider.is_err());
    }

    #[test]
    fn test_invalid_reputation() {
        let node_provider = NodeProvider::new("Node1".to_string(), vec![1, 2, 3], -1);
        assert!(node_provider.is_err());
    }

    #[test]
    fn test_getters() {
        let node_provider = NodeProvider::new("Node1".to_string(), vec![1, 2, 3], 10).unwrap();
        assert_eq!(node_provider.get_name(), "Node1");
        assert_eq!(node_provider.get_pubkey(), &[1, 2, 3]);
        assert_eq!(node_provider.get_reputation(), 10);
    }
}
