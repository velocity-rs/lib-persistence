use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    sync::OnceLock,
};

use log::{debug, error, info};
use mongodb::{
    Client, ClientSession, Collection, Database,
    bson::{self, Document},
    options::SessionOptions,
};

use crate::provider::Provider;
use serde::Serialize;

use serde_json::Value;

use super::errors::{MongoError, RepositoryError};

pub type Result<T> = std::result::Result<T, RepositoryError>;
pub type ObjectId = bson::oid::ObjectId;

static CLIENT: OnceLock<Client> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct Repository {
    pub(super) collection: Collection<Document>,

    #[expect(unused)]
    pub(super) database: Database,

    #[expect(unused)]
    pub(super) page_size: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepositoryConfig {
    pub(crate) primary_mongo_url: String,
    _secondary_mongo_url: Option<String>,
    pub(crate) app_name: String,
    pub database_name: String,
    pub collection_name: String,
    pub write_concern: Option<u32>,
    pub max_pool_size: Option<u32>,
    pub min_pool_size: Option<u32>,
    pub max_idle_time_secs: Option<u32>,
}

impl Display for RepositoryConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "RepositoryConfig {{ primary_mongo_url: {}, secondary_mongo_url: {:?}, app_name: {}, database_name: {}, collection_name: {}, write_concern: {:?}, max_pool_size: {:?}, min_pool_size: {:?}, max_idle_time_secs: {:?} }}",
            self.primary_mongo_url,
            self._secondary_mongo_url,
            self.app_name,
            self.database_name,
            self.collection_name,
            self.write_concern,
            self.max_pool_size,
            self.min_pool_size,
            self.max_idle_time_secs
        )
    }
}

impl RepositoryConfig {
    pub fn builder() -> RepositoryConfigBuilder {
        RepositoryConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct RepositoryConfigBuilder {
    primary_mongo_url: Option<String>,
    secondary_mongo_url: Option<String>,
    app_name: Option<String>,
    database_name: Option<String>,
    collection_name: Option<String>,
    write_concern: Option<u32>,
    max_pool_size: Option<u32>,
    min_pool_size: Option<u32>,
    max_idle_time_secs: Option<u32>,
}

impl RepositoryConfigBuilder {
    pub fn primary_mongo_url(mut self, url: impl Into<String>) -> Self {
        self.primary_mongo_url = Some(url.into());
        self
    }

    pub fn secondary_mongo_url(mut self, url: impl Into<String>) -> Self {
        self.secondary_mongo_url = Some(url.into());
        self
    }

    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    pub fn database_name(mut self, name: impl Into<String>) -> Self {
        self.database_name = Some(name.into());
        self
    }

    pub fn collection_name(mut self, name: impl Into<String>) -> Self {
        self.collection_name = Some(name.into());
        self
    }

    pub fn write_concern(mut self, concern: u32) -> Self {
        self.write_concern = Some(concern);
        self
    }

    pub fn max_pool_size(mut self, size: u32) -> Self {
        self.max_pool_size = Some(size);
        self
    }

    pub fn min_pool_size(mut self, size: u32) -> Self {
        self.min_pool_size = Some(size);
        self
    }

    pub fn max_idle_time_secs(mut self, time: u32) -> Self {
        self.max_idle_time_secs = Some(time);
        self
    }

    pub fn build(self) -> RepositoryConfig {
        RepositoryConfig {
            primary_mongo_url: self
                .primary_mongo_url
                .expect("primary_mongo_url is required"),
            _secondary_mongo_url: self.secondary_mongo_url,
            app_name: self.app_name.unwrap_or_default(),
            database_name: self.database_name.expect("database_name is required"),
            collection_name: self.collection_name.expect("collection_name is required"),
            write_concern: self.write_concern,
            max_pool_size: self.max_pool_size,
            min_pool_size: self.min_pool_size,
            max_idle_time_secs: self.max_idle_time_secs,
        }
    }
}

impl Repository {
    pub fn get_summary(&self) -> String {
        let ns = self.collection.namespace();
        ns.db + ":" + &ns.coll
    }

    pub async fn init(repository_config: &RepositoryConfig) -> Result<Repository> {
        match Provider::init(repository_config).await {
            Ok(_) => info!("Provider initialized"),
            Err(e) => {
                error!("Error initializing provider {}", e);
                return Err(RepositoryError::InitializationFailed);
            }
        }

        info!("Initializing repository");
        let client = match Provider::get_provider() {
            Ok(provider) => provider.get_client(),
            Err(e) => {
                error!("Error getting provider client {}", e);
                return Err(RepositoryError::InitializationFailed);
            }
        };

        let database = client.database(&repository_config.database_name);
        let collection: Collection<Document> =
            database.collection(&repository_config.collection_name);
        let repo = Repository {
            database,
            collection,
            page_size: 30,
        };
        info!(
            "Repository connected to DB: {}, Collection: {}",
            repository_config.database_name, repository_config.collection_name
        );

        Ok(repo)
    }

    pub async fn get_client() -> Result<&'static Client> {
        return match CLIENT.get() {
            Some(client) => Ok(client),
            None => Err(RepositoryError::UnIntialized),
        };
    }

    pub async fn get_collection(db_name: &str, coll_name: &str) -> Result<Collection<Document>> {
        let db = if let Some(client) = CLIENT.get() {
            client.database(db_name)
        } else {
            return Err(RepositoryError::UnIntialized);
        };

        Ok(db.collection(coll_name))
    }

    pub async fn get(repository_config: RepositoryConfig) -> Result<Repository> {
        debug!("Getting repository from provider client(s)");
        let client = match CLIENT.get() {
            Some(client) => client,
            None => {
                error!("Provider is not initialized");
                return Err(RepositoryError::InitializationFailed);
            }
        };

        let database = client.database(&repository_config.database_name);

        match database.list_collection_names().await {
            Ok(_) => info!("Client is connected to DB. Can proceeed."),
            Err(e) => {
                error!("Error connecting to the DB cluster {}", e);
                return Err(RepositoryError::ClientError(MongoError::from(e)));
            }
        }

        let collection: Collection<Document> =
            database.collection(&repository_config.collection_name);
        Ok(Repository {
            database,
            collection,
            page_size: 30,
        })
    }

    pub async fn get_session() -> Result<ClientSession> {
        if CLIENT.get().is_none() {
            return Err(RepositoryError::UnIntialized);
        }

        let client = match CLIENT.get() {
            Some(client) => client,
            None => {
                return Err(RepositoryError::InitializationFailed);
            }
        };

        let session_options = SessionOptions::builder().causal_consistency(false).build();

        return match client.start_session().with_options(session_options).await {
            Ok(session) => Ok(session),
            Err(e) => Err(RepositoryError::SessionCreationError(MongoError::from(e))),
        };
    }

    pub(super) fn convert_datetime(value: &mut Value, key: &str) {
        if let Some(bson_dt) = value[key]["$date"].as_str() {
            value[key] = Value::String(bson_dt.to_string());
        }
    }
}
