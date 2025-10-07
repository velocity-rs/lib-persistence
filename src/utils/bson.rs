use log::error;
use mongodb::bson::{self, Document};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

type SeError = mongodb::bson::ser::Error;
type DeError = mongodb::bson::de::Error;

pub fn get_doc_from_value(value: &Value) -> Result<Document, SeError> {
    return match bson::to_document(value) {
        Ok(d) => Ok(d),
        Err(e) => {
            error!("Error getting bson from value {}", e);
            Err(e)
        }
    };
}

pub fn get_value_from_doc(doc: Document) -> Result<Value, DeError> {
    return match bson::from_document(doc) {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("Error getting value from bson {}", e);
            Err(e)
        }
    };
}

#[expect(unused)]
pub fn to_doc<T>(t: &T) -> Result<Document, SeError>
where
    T: Serialize,
{
    return match bson::to_document(t) {
        Ok(d) => Ok(d),
        Err(e) => {
            error!("Error getting bson from value {}", e);
            Err(e)
        }
    };
}

pub fn from_doc<T>(doc: Document) -> Result<T, DeError>
where
    T: DeserializeOwned,
{
    return match bson::from_document(doc) {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("Error getting value from bson {}", e);
            Err(e)
        }
    };
}
