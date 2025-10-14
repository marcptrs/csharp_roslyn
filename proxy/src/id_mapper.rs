use crate::message::MessageId;
use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct IdMapper {
    next_id: AtomicI64,
    client_to_server: DashMap<MessageId, MessageId>,
    server_to_client: DashMap<MessageId, MessageId>,
}

impl IdMapper {
    pub fn new() -> Self {
        Self {
            next_id: AtomicI64::new(1),
            client_to_server: DashMap::new(),
            server_to_client: DashMap::new(),
        }
    }

    pub fn map_client_id(&self, client_id: MessageId) -> MessageId {
        if let Some(server_id) = self.client_to_server.get(&client_id) {
            return server_id.clone();
        }

        let server_id = MessageId::Number(self.next_id.fetch_add(1, Ordering::SeqCst));

        self.client_to_server
            .insert(client_id.clone(), server_id.clone());
        self.server_to_client
            .insert(server_id.clone(), client_id);

        server_id
    }

    pub fn get_client_id(&self, server_id: &MessageId) -> Option<MessageId> {
        self.server_to_client.get(server_id).map(|r| r.clone())
    }

    pub fn remove(&self, server_id: &MessageId) {
        if let Some((_, client_id)) = self.server_to_client.remove(server_id) {
            self.client_to_server.remove(&client_id);
        }
    }
}

impl Default for IdMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_mapping_roundtrip() {
        let mapper = IdMapper::new();

        let client_id = MessageId::Number(42);
        let server_id = mapper.map_client_id(client_id.clone());

        assert_ne!(client_id, server_id);

        let retrieved_client_id = mapper.get_client_id(&server_id).unwrap();
        assert_eq!(client_id, retrieved_client_id);
    }

    #[test]
    fn test_id_mapping_consistency() {
        let mapper = IdMapper::new();

        let client_id = MessageId::String("test-123".to_string());
        let server_id_1 = mapper.map_client_id(client_id.clone());
        let server_id_2 = mapper.map_client_id(client_id.clone());

        assert_eq!(server_id_1, server_id_2);
    }

    #[test]
    fn test_id_removal() {
        let mapper = IdMapper::new();

        let client_id = MessageId::Number(100);
        let server_id = mapper.map_client_id(client_id.clone());

        mapper.remove(&server_id);

        assert!(mapper.get_client_id(&server_id).is_none());
    }

    #[test]
    fn test_unique_server_ids() {
        let mapper = IdMapper::new();

        let client_id_1 = MessageId::Number(1);
        let client_id_2 = MessageId::Number(2);

        let server_id_1 = mapper.map_client_id(client_id_1);
        let server_id_2 = mapper.map_client_id(client_id_2);

        assert_ne!(server_id_1, server_id_2);
    }
}
