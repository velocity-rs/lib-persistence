pub mod error;
mod options;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use error::CursorError;
use futures::stream::StreamExt;
use log::error;
use mongodb::bson;
use mongodb::bson::{Bson, Document, doc, oid::ObjectId};
use mongodb::options::{CountOptions, EstimatedDocumentCountOptions};
use mongodb::{Collection, options::FindOptions};
use options::CursorOptions;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Neg;

/// Provides details about if there are more pages and the cursor to the start of the list and end
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PaginatedResults {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub total_results: u64,
    pub page_size: i64,
    pub current_page: u64,
    pub start_cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub results: Vec<Value>,
}

/*
#[cfg(feature = "graphql")]
#[juniper::object]
impl PageInfo {
    fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    fn has_previous_page(&self) -> bool {
        self.has_previous_page
    }

    fn start_cursor(&self) -> Option<String> {
        self.start_cursor.to_owned()
    }

    fn next_cursor(&self) -> Option<String> {
        self.next_cursor.to_owned()
    }
}

/// Edges are the cursors on all of the items in the return


#[cfg(feature = "graphql")]
#[juniper::object]
impl Edge {
    fn cursor(&self) -> String {
        self.cursor.to_owned()
    }
}
// FIX: there's probably a better way to do this...but for now
#[cfg(feature = "graphql")]
impl From<&Edge> for Edge {
    fn from(edge: &Edge) -> Edge {
        Edge {
            cursor: edge.cursor.clone(),
        }
    }
}
    */

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Edge {
    pub cursor: String,
}
/// The result of a find method with the items, edges, pagination info, and total count of objects
#[derive(Debug, Default)]
pub struct FindResult<T> {
    pub page_info: PageInfo,
    #[expect(unused)]
    pub edges: Vec<Edge>,
    pub total_count: u64,
    pub items: Vec<T>,
}

/// The direction of the list, ie. you are sending a cursor for the next or previous items. Defaults to Next
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CursorDirections {
    Previous,
    Next,
}

/// The main entry point for finding documents
#[derive(Debug)]
pub struct PaginatedCursor {
    has_cursor: bool,
    cursor_doc: Document,
    direction: CursorDirections,
    options: CursorOptions,
}

impl PaginatedCursor {
    /// Updates or creates all of the find options to help with pagination and returns a `PaginatedCursor` object.
    ///
    /// # Arguments
    /// * `options` - Optional find options that you would like to perform any searches with
    /// * `cursor` - An optional existing cursor in base64. This would have come from a previous `FindResult<T>`
    /// * `direction` - Determines whether the cursor supplied is for a previous page or the next page. Defaults to Next
    ///
    #[must_use]
    pub fn new(
        options: Option<FindOptions>,
        cursor: Option<String>,
        direction: Option<CursorDirections>,
    ) -> Self {
        Self {
            // parse base64 for keys
            has_cursor: cursor.is_some(),
            cursor_doc: cursor.map_or_else(Document::new, |b64| {
                map_from_base64(b64).expect("Unable to parse cursor")
            }),
            direction: direction.unwrap_or(CursorDirections::Next),
            options: CursorOptions::from(options.unwrap_or_default()),
        }
    }

    /// Estimates the number of documents in the collection using collection metadata.
    #[expect(unused)]
    pub async fn estimated_document_count<T>(
        &self,
        collection: &Collection<T>,
    ) -> Result<u64, CursorError>
    where
        T: Sync + Send,
    {
        let total_count = collection
            .estimated_document_count()
            .with_options(Some(EstimatedDocumentCountOptions::from(
                self.options.clone(),
            )))
            .await
            .unwrap();
        Ok(total_count)
    }

    /// Gets the number of documents matching filter.
    /// Note that using [`PaginatedCursor::estimated_document_count`](#method.estimated_document_count)
    /// is recommended instead of this method is most cases.
    pub async fn count_documents<T>(
        &self,
        collection: &Collection<T>,
        query: Option<&Document>,
    ) -> Result<u64, CursorError>
    where
        T: Sync + Send,
    {
        let mut count_options = self.options.clone();
        count_options.limit = None;
        count_options.skip = None;
        let count_query = query.map_or_else(Document::new, Clone::clone);
        let total_count = match collection
            .count_documents(count_query)
            .with_options(Some(CountOptions::from(count_options)))
            .await
        {
            Ok(count) => count,
            Err(e) => {
                error!("Error executing query");
                return Err(CursorError::MongoError(e));
            }
        };

        Ok(total_count)
    }
    /// Finds the documents in the `collection` matching `filter`.
    pub async fn find<T>(
        &self,
        collection: &Collection<Document>,
        filter: Option<&Document>,
    ) -> Result<FindResult<T>, CursorError>
    where
        T: DeserializeOwned + Sync + Send + Unpin + Clone,
    {
        // first count the docs
        let total_count = match self.count_documents(collection, filter).await {
            Ok(count) => count,
            Err(e) => {
                return Err(e);
            }
        };

        // setup defaults
        let mut items: Vec<T> = vec![];
        let mut edges: Vec<Edge> = vec![];

        #[expect(unused)]
        let mut has_next_page = false;

        #[expect(unused)]
        let mut has_previous_page = false;
        let mut has_skip = false;
        let mut start_cursor: Option<String> = None;
        let mut next_cursor: Option<String> = None;

        // return if we if have no docs
        if total_count == 0 {
            return Ok(FindResult {
                page_info: PageInfo::default(),
                edges: vec![],
                total_count: 0,
                items: vec![],
            });
        }

        // build the cursor
        let query_doc = self.get_query(filter.cloned());
        let mut options = self.options.clone();
        let skip_value = options.skip.unwrap_or(0);
        if self.has_cursor || skip_value == 0 {
            options.skip = None;
        } else {
            has_skip = true;
        }
        // let has_previous
        let is_previous_query = self.has_cursor && self.direction == CursorDirections::Previous;
        // if it's a previous query we need to reverse the sort we were doing
        if is_previous_query {
            if let Some(sort) = options.sort.as_mut() {
                sort.iter_mut().for_each(|(_key, value)| {
                    if let Bson::Int32(num) = value {
                        *value = Bson::Int32(num.neg());
                    }
                    if let Bson::Int64(num) = value {
                        *value = Bson::Int64(num.neg());
                    }
                });
            }
        }
        let mut cursor = collection
            .find(query_doc)
            .with_options(Some(options.into()))
            .await
            .unwrap();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => {
                    let item = bson::from_bson(Bson::Document(doc.clone())).unwrap();
                    edges.push(Edge {
                        cursor: self.create_from_doc(&doc),
                    });
                    items.push(item);
                }
                Err(error) => {
                    error!("Error finding doc: {}", error);
                }
            }
        }
        let has_more: bool;
        if has_skip {
            has_more = (items.len() as u64).saturating_add(skip_value) < total_count;
            has_previous_page = true;
            has_next_page = has_more;
        } else {
            has_more = items.len() as i64 > self.options.limit.unwrap().saturating_sub(1);
            has_previous_page = (self.has_cursor && self.direction == CursorDirections::Next)
                || (is_previous_query && has_more);
            has_next_page = (self.direction == CursorDirections::Next && has_more)
                || (is_previous_query && self.has_cursor);
        }

        // reorder if we are going backwards
        if is_previous_query {
            items.reverse();
            edges.reverse();
        }
        // remove the extra item to check if we have more
        if has_more && !is_previous_query {
            items.pop();
            edges.pop();
        } else if has_more {
            items.remove(0);
            edges.remove(0);
        }

        // create the next cursor
        if !items.is_empty() && edges.len() == items.len() {
            start_cursor = Some(edges[0].cursor.clone());
            next_cursor = Some(edges[items.len().saturating_sub(1)].cursor.clone());
        }

        let page_info = PageInfo {
            has_next_page,
            has_previous_page,
            start_cursor,
            next_cursor,
        };
        Ok(FindResult {
            page_info,
            edges,
            total_count,
            items,
        })
    }

    fn get_value_from_doc(&self, key: &str, doc: Bson) -> Option<(String, Bson)> {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        match doc {
            Bson::Document(d) => d.get(parts[0]).and_then(|value| match value {
                Bson::Document(d) => self.get_value_from_doc(parts[1], Bson::Document(d.clone())),
                _ => Some((parts[0].to_string(), value.clone())),
            }),
            _ => Some((parts[0].to_string(), doc)),
        }
    }

    fn create_from_doc(&self, doc: &Document) -> String {
        let mut only_sort_keys = Document::new();
        self.options.sort.as_ref().map_or_else(String::new, |sort| {
            for key in sort.keys() {
                if let Some((_, value)) = self.get_value_from_doc(key, Bson::Document(doc.clone()))
                {
                    only_sort_keys.insert(key, value);
                }
            }
            let buf = bson::to_vec(&only_sort_keys).unwrap();
            STANDARD.encode(buf)
        })
    }

    /*
    $or: [{
        launchDate: { $lt: nextLaunchDate }
    }, {
        // If the launchDate is an exact match, we need a tiebreaker, so we use the _id field from the cursor.
        launchDate: nextLaunchDate,
    _id: { $lt: nextId }
    }]
    */
    fn get_query(&self, query: Option<Document>) -> Document {
        // now create the filter
        let mut query_doc = query.unwrap_or_default();

        // Don't do anything if no cursor is provided
        if self.cursor_doc.is_empty() {
            return query_doc;
        }
        let Some(sort) = &self.options.sort else {
            return query_doc;
        };

        // this is the simplest form, it's just a sort by _id
        if sort.len() <= 1 {
            let object_id = self.cursor_doc.get("_id").unwrap().clone();
            let direction = self.get_direction_from_key(sort, "_id");
            query_doc.insert("_id", doc! { direction: object_id });
            return query_doc;
        }

        let mut queries: Vec<Document> = Vec::new();
        let mut previous_conditions: Vec<(String, Bson)> = Vec::new();

        // Add each sort condition with it's direction and all previous condition with fixed values
        for key in sort.keys() {
            let mut query = query_doc.clone();
            query.extend(previous_conditions.clone().into_iter()); // Add previous conditions

            let value = self.cursor_doc.get(key).unwrap_or(&Bson::Null);
            let direction = self.get_direction_from_key(sort, key);
            query.insert(key, doc! { direction: value.clone() });
            previous_conditions.push((key.clone(), value.clone())); // Add self without direction to previous conditions

            queries.push(query);
        }

        query_doc = if queries.len() > 1 {
            doc! { "$or": queries.iter().as_ref() }
        } else {
            queries.pop().unwrap_or_default()
        };
        query_doc
    }

    fn get_direction_from_key(&self, sort: &Document, key: &str) -> &'static str {
        let value = sort.get(key).and_then(Bson::as_i32).unwrap_or(0);
        match self.direction {
            CursorDirections::Next => {
                if value >= 0 {
                    "$gt"
                } else {
                    "$lt"
                }
            }
            CursorDirections::Previous => {
                if value >= 0 {
                    "$lt"
                } else {
                    "$gt"
                }
            }
        }
    }
}

fn map_from_base64(base64_string: String) -> Result<Document, CursorError> {
    // change from base64
    let decoded = STANDARD.decode(base64_string)?;
    // decode from bson
    let cursor_doc = bson::from_slice(decoded.as_slice()).unwrap();
    Ok(cursor_doc)
}
#[expect(unused)]
/// Converts an id into a `MongoDb` `ObjectId`
pub fn get_object_id(id: &str) -> Result<ObjectId, CursorError> {
    let object_id = match ObjectId::parse_str(id) {
        Ok(object_id) => object_id,
        Err(_e) => return Err(CursorError::InvalidId(id.to_string())),
    };
    Ok(object_id)
}
