use log::error;
use serde_json::Value;

use crate::Repository;

use mongodb::{bson, options::FullDocumentType};

impl Repository {
    pub async fn watch(&self, handler: fn(updated_value: Option<Value>)) {
        let messages_coll = &self.collection;

        let mut change_stream = messages_coll
            .watch()
            .full_document(FullDocumentType::UpdateLookup)
            .await
            .unwrap();
        while change_stream.is_alive() {
            if let Some(event) = change_stream.next_if_any().await.unwrap() {
                let bson_doc = event.full_document.unwrap();

                match bson::from_document(bson_doc) {
                    Ok(d) => handler(Some(d)),
                    Err(e) => {
                        error!("Error getting value from bson {}", e);
                        handler(None);
                    }
                }
            }
        }
    }
}
