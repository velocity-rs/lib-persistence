use async_once::AsyncOnce;
use lazy_static::lazy_static;

use crate::config;
use log::{error, info, trace};
use mongodb::{options::ClientOptions, Client, Database, Collection};

use super::errors::DBError;

lazy_static! {
    static ref MONGO_SCHEMA_CLIENT: AsyncOnce<Option<Client>> = AsyncOnce::new(async {
        info!("Getting Mongo client");
        //Get configs
        let data_mongo_url: String = match config::get::<String>("config_mongo_cloud_url") {
            Some(url) => url,
            None => panic!("Mongo DATA url not found...cannot continue"),
        };
        let app_name = config::get::<String>("app_name").unwrap_or("velocity-lib".to_string());

        // Setup client options
        let client_options: ClientOptions = match ClientOptions::parse(data_mongo_url).await {
            Ok(mut c) => {
                c.app_name = Some(app_name); //TODO move this to arg.
                c
            }
            Err(e) => {
                error!("Unable to get db connection {}", e);
                return None
            }
        };

        let client = match Client::with_options(client_options) {
            Ok(client) => client,
            Err(e) => {
                error!("Could not connect to db {}", e);
                return None
            }
        };
        info!("Database client created successfully");
        Some(client)
    });
}

pub async fn get_config_db() -> Result<Database, DBError> {
    info!("Getting Database");
    match MONGO_SCHEMA_CLIENT.get().await {
        Some(client) => {
            let mongo_db_name: &String = &config::get::<String>("config_mongo_db_name")
                .expect("Mongo Database Name not set MONGO_DB_NAME");
            Ok(client.to_owned().database(mongo_db_name))
        }
        None => Err(DBError::UnexpectedError),
    }
}

pub async fn get_config_client() -> Result<Client, DBError> {
    //TODO handle error
    info!("Getting Mongo client");
    match MONGO_SCHEMA_CLIENT.get().await {
        Some(c) => Ok(c.to_owned()),
        None => Err(DBError::UnexpectedError),
    }
}


pub async fn get_config_collection<T>(coll_name: &str) -> Result<Collection<T>, DBError> {
    match get_config_db().await {
        Ok(db) => Ok(db.collection(coll_name)),
        Err(e) => Err(e),
    }
}
