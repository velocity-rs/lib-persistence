use crate::{
    MongoError, PaginatedResults, Repository, RepositoryError, Result,
    pagination::{FindResult, PaginatedCursor},
    utils,
};
use futures::stream::StreamExt;
use log::trace;
use mongodb::{Collection, bson::Document, options::FindOptions};
use mongodb::{
    bson::doc,
    options::{CountOptions, FindOneOptions, ReadConcern},
};
use serde_json::Value;

impl Repository {
    pub async fn count(&self, count_filter: &Value) -> Result<u64> {
        let filter_doc = match utils::bson::to_doc(count_filter) {
            Ok(doc) => doc,
            Err(e) => {
                return Err(RepositoryError::BsonSerializationFailed(e.to_string()));
            }
        };
        let count_options = CountOptions::builder()
            .read_concern(ReadConcern::majority())
            .build();

        match self
            .collection
            .count_documents(filter_doc)
            .with_options(count_options)
            .await
        {
            Ok(count) => {
                return Ok(count);
            }
            Err(e) => {
                return Err(RepositoryError::CountFailed(e.to_string()));
            }
        }
    }

    pub async fn find_record_ids(&self, find_filter: &Document) -> Result<Vec<String>> {
        let start = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();
        trace!(
            "Find only record_ids filter {} in Repository {}",
            find_filter,
            self.collection.name()
        );
        let p = doc! { "_id": 0 , "_record_id": 1 };
        let find_options = FindOptions::builder().projection(p).build();
        let mut result: Vec<String> = vec![];
        match self
            .collection
            .find(find_filter.to_owned())
            .with_options(find_options)
            .await
        {
            Ok(mut cursor) => {
                while let Some(doc) = cursor.next().await {
                    let doc = doc.unwrap();
                    let rid = doc.get("_record_id").unwrap().as_str().unwrap();
                    result.push(rid.to_owned());
                }
            }
            Err(_) => todo!(),
        }

        let end = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();

        trace!("Time taken to get record_ids {}", end - start);

        Ok(result)
    }

    pub async fn find_one(&self, find_filter: &Value, projection: Option<&Value>) -> Result<Value> {
        let collection: &Collection<Document> = &self.collection;
        trace!(
            "Find filter {} in Repository {}",
            find_filter,
            self.collection.name()
        );

        let filter_doc = match utils::bson::to_doc(find_filter) {
            Ok(doc) => doc,
            Err(e) => {
                return Err(RepositoryError::BsonSerializationFailed(e.to_string()));
            }
        };
        let projection_doc = match projection {
            Some(p) => match utils::bson::to_doc(p) {
                Ok(doc) => Some(doc),
                Err(e) => {
                    return Err(RepositoryError::BsonSerializationFailed(e.to_string()));
                }
            },
            None => None,
        };

        let find_one_options = FindOneOptions::builder()
            .projection(projection_doc)
            .read_concern(ReadConcern::majority())
            .build();

        return match collection
            .find_one(filter_doc)
            .with_options(find_one_options)
            .await
        {
            Ok(result) => {
                if let Some(d) = result {
                    // Convert BSON Document to serde_json::Value
                    let v = utils::bson::from_doc::<Value>(d).unwrap();
                    Ok(v)
                } else {
                    Err(RepositoryError::NoDocument)
                }
            }
            Err(e) => Err(RepositoryError::FindFailed(MongoError::from(e))),
        };
    }

    pub async fn find(
        &self,
        find_filter: &Document,
        mut options: FindOptions,
    ) -> Result<PaginatedResults> {
        let collection: &Collection<Document> = &self.collection;

        trace!(
            "Find filter {} in Repository {}",
            find_filter,
            self.collection.name()
        );
        trace!("Find options {:?}", options);

        options.projection = if let Some(mut projection) = options.projection {
            projection.insert("_id", 0);
            Some(projection)
        } else {
            let p = doc! { "_id": 0 };
            Some(p)
        };
        let paginated_cursor = PaginatedCursor::new(Some(options.clone()), None, None);

        let find_results: FindResult<Document> = match paginated_cursor
            .find(collection.into(), Some(find_filter))
            .await
        {
            Ok(results) => results,
            Err(e) => {
                return Err(RepositoryError::FindFailed(MongoError::from(e)));
            }
        };

        let page_size = match options.limit {
            Some(page_size) => {
                if page_size < 1 {
                    1
                } else {
                    page_size
                }
            }
            None => 1,
        };

        let skip = options.skip.unwrap_or(0);
        let current_page = skip / (page_size as u64);

        let result_rids: Vec<Value> = find_results
            .items
            .iter()
            .map(|d| {
                let mut val = utils::bson::from_doc::<Value>(d.clone()).unwrap();
                Self::convert_datetime(&mut val, "created_at");
                Self::convert_datetime(&mut val, "updated_at");
                val
            })
            .collect();

        let paginated_results = PaginatedResults {
            has_next_page: find_results.page_info.has_next_page,
            has_previous_page: find_results.page_info.has_previous_page,
            start_cursor: find_results.page_info.start_cursor,
            next_cursor: find_results.page_info.next_cursor,
            total_results: find_results.total_count,
            page_size: page_size,
            current_page: current_page,
            results: result_rids,
        };

        Ok(paginated_results)
    }

    /*pub async fn find_one(
        &self,
        find_filter: Filter,
        options: Option<FindOneOptions>
    ) -> Result<Option<Document>> {
        debug!(
            "Finding records with filter {:?} in {:?} in db {}",
            find_filter,
            self.collection.name(),
            self.collection.namespace()
        );
        let mut find_one_options = match options {
            Some(opt) => { opt }
            None => { FindOneOptions::default() }
        };
        find_one_options.max_time = Some(Duration::from_millis(500));

        let find_results = self.collection.find_one(find_filter, Some(find_one_options)).await;

        match find_results {
            Ok(doc) => {
                trace!("Got document from repo {:?}", doc);
                Ok(doc)
            }
            Err(e) => {
                error!("Error getting result for find one {}", e);
                Err(RepositoryError::FindFailed(MongoError::from(e)))
            }
        }
    }

    pub async fn find_many_by_rids(&self, rids: Vec<String>) {
        debug!("Find by record_ids {:?}", rids);

        let rid_doc_map: HashMap<String, String> = rids
            .iter()
            .map(|rid| { ("_record_id".to_owned(), rid.clone()) })
            .collect();

        //let mut or_map: HashMap<String, HashMap<String, String>> = HashMap::new();
        //or_map.insert("$or".to_string(), rid_doc_map);

        //trace!("Or Map {:?}", or_map);

        let filter = bson::to_bson(&rid_doc_map).unwrap();
        let or_filter = bson!({"$or": filter});
        let filter_doc = or_filter.as_document().unwrap();

        let results = self.find(Some(filter_doc.clone()), None).await;

        trace!("Results {:?}", results);
    }*/
}
