use log::{error, trace};

use serde_json::Value;

use crate::utils;
use crate::{MongoError, Repository, RepositoryError, Result};
use mongodb::{
    bson::Document,
    options::{FindOneAndUpdateOptions, ReturnDocument, UpdateModifications, WriteConcern},
};

impl Repository {
    pub async fn update_one(
        &self,
        filter: &Document,
        update: &Value,
        options: Option<FindOneAndUpdateOptions>,
    ) -> Result<Option<Value>> {
        trace!("Find one and update");

        let update_doc = match utils::bson::get_doc_from_value(&update) {
            Ok(doc) => doc,
            Err(e) => {
                error!("Error getting mongo update doc from value");
                return Err(RepositoryError::BsonSerializationFailed(e.to_string()));
            }
        };
        let update = UpdateModifications::Document(update_doc);

        let options = match options {
            Some(opts) => opts,
            None => FindOneAndUpdateOptions::builder()
                .upsert(Some(false))
                .write_concern(Some(WriteConcern::majority()))
                .return_document(ReturnDocument::After)
                .build(),
        };

        let update_result = self
            .collection
            .find_one_and_update(filter.clone(), update)
            .with_options(options)
            .await;

        match update_result {
            Ok(res) => match res {
                Some(r) => match utils::bson::get_value_from_doc(r) {
                    Ok(v) => Ok(Some(v)),
                    Err(e) => {
                        error!("Error deserializing update result");
                        Err(RepositoryError::BsonDeSerializationFailed(e.to_string()))
                    }
                },
                None => Ok(None),
            },
            Err(e) => Err(RepositoryError::UpdateOneFailed(MongoError::from(e))),
        }
    }
}
