//! MongoDB provider for repository access.
//!
//! This module defines the [`Provider`] struct, which manages a singleton MongoDB client
//! and provides initialization and access methods for use throughout the application.
//!
//! # Usage
//!
//! - Call [`Provider::init`] once at startup with a [`RepositoryConfig`] to initialize the global provider.
//! - Use [`Provider::get_provider`] to retrieve the singleton provider instance.
//! - Use [`Provider::get_client`] to get a clone of the underlying MongoDB client.
//!
//! # Example
//! ```rust,ignore
//! let config = RepositoryConfig::builder()
//!     .primary_mongo_url("mongodb://localhost:27017")
//!     .app_name("my_app")
//!     .database_name("my_db")
//!     .collection_name("my_collection")
//!     .build();
//! Provider::init(&config).await?;
//! let provider = Provider::get_provider()?;
//! let client = provider.get_client();
//! ```

use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

use log::{debug, error, info, trace};
use mongodb::{
    Client,
    bson::doc,
    options::{
        Acknowledgment::Nodes, ClientOptions, ReadConcernLevel, ServerApi, ServerApiVersion,
        WriteConcern,
    },
};

use crate::{
    RepositoryConfig,
    errors::{MongoError, RepositoryError},
    provider,
};

static PROVIDER: OnceLock<Mutex<Provider>> = OnceLock::new();

/// Singleton MongoDB provider for repository access.
///
/// Holds a MongoDB [`Client`] and provides methods for initialization and retrieval.
#[derive(Clone, Debug)]
pub struct Provider {
    client: Client,
}

impl Provider {
    /// Initializes the global provider with the given repository configuration.
    ///
    /// Sets up the MongoDB client with connection options, write concern, read concern,
    /// and connection pool settings. Pings the database to verify connectivity.
    ///
    /// Returns a reference to the singleton [`Provider`] on success.
    pub async fn init(repository_config: &RepositoryConfig) -> Result<(), RepositoryError> {
        let conn_url = repository_config.primary_mongo_url.clone();
        debug!("Parsing connection string into ClientOptions");
        let mut client_options = match ClientOptions::parse(conn_url).await {
            Ok(client_options) => client_options,
            Err(e) => {
                error!("Error parsing connection url {}", e);
                return Err(RepositoryError::InitializationFailed);
            }
        };
        let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
        client_options.server_api = Some(server_api);
        client_options.app_name = Some(repository_config.app_name.to_string());

        // Add write concern if specified, defaults to 3 assuming a 3 node replicaset.
        let wc = repository_config.write_concern.unwrap_or(3);
        let write_concern = WriteConcern::builder().w(Nodes(wc)).build();
        client_options.write_concern = Some(write_concern);

        client_options.read_concern = Some(ReadConcernLevel::Majority.into());
        client_options.max_pool_size = repository_config.max_pool_size;
        client_options.min_pool_size = repository_config.min_pool_size;
        client_options.max_idle_time = repository_config
            .max_idle_time_secs
            .map(|secs| Duration::from_secs(secs as u64));

        trace!("Client options: {:?}", client_options);

        match Client::with_options(client_options) {
            Ok(client) => {
                info!("Created MongoDB client");
                // Ping the database to verify connectivity in a separate thread

                let provider = Provider { client };

                match PROVIDER.set(Mutex::new(provider)) {
                    Ok(_) => {
                        info!("MongoDB provider initialized successfully");
                        // Start the ping test in a separate async task
                        tokio::spawn(Self::start_ping_test());
                        return Ok(());
                    }
                    Err(_) => {
                        error!("MongoDB provider has already been initialized");
                        return Err(RepositoryError::InitializationFailed);
                    }
                }
            }
            Err(e) => {
                error!("Error creating MongoDB client: {}", e);
                return Err(RepositoryError::InitializationFailed);
            }
        }
    }

    /// Returns a clone of the singleton [`Provider`] if initialized.
    ///
    /// Returns an error if the provider has not been initialized.
    pub fn get_provider() -> Result<Provider, RepositoryError> {
        match PROVIDER.get() {
            Some(provider) => Ok(provider.lock().unwrap().clone()),
            None => Err(RepositoryError::UnIntialized),
        }
    }

    /// Returns a clone of the underlying MongoDB [`Client`].
    pub fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn start_ping_test() {
        // clone the client so the spawned tasks own their client handle
        let client = PROVIDER.get().unwrap().lock().unwrap().client.clone();
        // wait for 30 seconds before starting the ping test to allow initial connections to establish
        tokio::time::sleep(Duration::from_secs(30)).await;
        info!("Starting MongoDB ping test task");
        // run a ping test in an endless loop
        loop {
            let client = client.clone();
            tokio::spawn(async move {
                match client
                    .database("admin")
                    .run_command(doc! {"ping": 1})
                    .with_options(None)
                    .await
                {
                    Ok(_) => info!("Pinged deployment. Successfully connected to MongoDB!"),
                    Err(e) => {
                        error!(
                            "Could not ping DB cluster. This could be due a configuration issue {}",
                            e
                        );
                    }
                }
            });
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
