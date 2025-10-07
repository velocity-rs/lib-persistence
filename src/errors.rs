use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::pagination::error::CursorError;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DBError {
    #[error("Unexpected error in DB")]
    UnexpectedError,
    #[error("Database Connection Error {}", 0)]
    DBConnectionError(String),
}

#[derive(Debug)]
pub struct MongoError {
    kind: String,
    message: String,
}

impl From<mongodb::error::Error> for MongoError {
    fn from(e: mongodb::error::Error) -> Self {
        return MongoError {
            kind: e.kind.to_string(),
            message: e.to_string(),
        };
    }
}

impl From<CursorError> for MongoError {
    fn from(e: CursorError) -> Self {
        return MongoError {
            kind: "CursorError".to_string(),
            message: e.to_string(),
        };
    }
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Bson Serialization Failed {0}")]
    BsonSerializationFailed(String),
    #[error("Bson DeSerialization Failed {0}")]
    BsonDeSerializationFailed(String),
    #[error("Error updating document {}: {}", .0.kind, .0.message)]
    UpdateOneFailed(MongoError),
    #[error("Error inserting document {}: {}", .0.kind, .0.message)]
    InsertOneFailed(MongoError),
    #[error("Error inserting documents {}: {}", .0.kind, .0.message)]
    InsertManyFailed(MongoError),
    #[error("A client error has occured {}: {}", .0.kind, .0.message)]
    ClientError(MongoError),
    #[error("A database error has occured {}: {}", .0.kind, .0.message)]
    DatabaseError(MongoError),
    #[error("DB repository is unusable because its not initialized. HINT: Use the init() function")]
    UnIntialized,
    #[error("Repository initialization failed")]
    InitializationFailed,
    #[error("There are no documents in the result")]
    NoDocument,
    #[error("Error while running find command {}: {}", .0.kind, .0.message)]
    FindFailed(MongoError),
    #[error("Error during watch {}: {}", .0.kind, .0.message)]
    WatchError(MongoError),
    #[error("Error starting session {}: {}", .0.kind, .0.message)]
    SessionCreationError(MongoError),
    #[error("Error creating index {}: {}", .0.kind, .0.message)]
    IndexCreationError(MongoError),
    #[error("Could not start transaction {0}")]
    FailedStartingTransaction(String),
    #[error("Count failed {0}")]
    CountFailed(String),
    #[error("Repository is already initialized")]
    AlreadyInitialized,
}
