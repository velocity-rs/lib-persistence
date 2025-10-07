use log::{error, trace};
use mongodb::{
    bson::{self, Document},
    options::{InsertManyOptions, WriteConcern},
};
use serde_json::Value;

use crate::utils;
use crate::{MongoError, ObjectId, Repository, RepositoryError, Result};

impl Repository {
    pub async fn insert_one(&self, doc: &mut Document) -> Result<ObjectId> {
        trace!("Inserting a document into repo");

        let now = bson::DateTime::now();

        doc.insert("created_at", now);
        doc.insert("updated_at", now);

        let insert_one_result = self.collection.insert_one(doc).await;

        match insert_one_result {
            Ok(result) => {
                trace!("Created document {}", result.inserted_id);
                Ok(result.inserted_id.as_object_id().unwrap())
            }
            Err(e) => {
                error!("Error creating document {}", e);
                return Err(RepositoryError::InsertOneFailed(MongoError::from(e)));
            }
        }
    }

    pub async fn insert_many(&self, values: Vec<Value>) -> Result<Vec<ObjectId>> {
        trace!("Insert many documents into repo");

        let now = bson::DateTime::now();

        let mut docs_with_times: Vec<Document> = Vec::new();

        for value in values {
            let mut doc = match utils::bson::get_doc_from_value(&value) {
                Ok(d) => d,
                Err(e) => {
                    return Err(RepositoryError::BsonSerializationFailed(e.to_string()));
                }
            };

            doc.insert("created_at", now);
            doc.insert("updated_at", now);
            docs_with_times.push(doc);
        }

        let insert_many_options = InsertManyOptions::builder()
            .ordered(Some(true))
            .write_concern(Some(WriteConcern::majority()))
            .build();

        let insert_many = self
            .collection
            .insert_many(docs_with_times)
            .with_options(insert_many_options);

        match insert_many.await {
            Ok(oids) => {
                let mut result: Vec<ObjectId> = Vec::new();
                for oid in oids.inserted_ids.values() {
                    let obj_id = oid.as_object_id().unwrap();
                    result.push(obj_id);
                }
                return Ok(result);
            }
            Err(e) => {
                error!("Error creating documents {}", e);
                return Err(RepositoryError::InsertManyFailed(MongoError::from(e)));
            }
        }
    }
}
