use log::error;
use serde::{Serialize, de::DeserializeOwned};

#[expect(unused)]
pub fn to_bytes<T>(data: &T) -> Option<Vec<u8>>
where
    T: Serialize,
{
    let mut bytes: Vec<u8> = Vec::new();
    match serde_json::to_writer(&mut bytes, data) {
        Ok(()) => Some(bytes),
        Err(e) => {
            error!("Error serializing data to bytes: {}", e);
            None
        }
    }
}

#[expect(unused)]
pub fn from_bytes<T>(bytes: &Vec<u8>) -> Option<T>
where
    T: DeserializeOwned,
{
    match serde_json::from_slice::<T>(&bytes) {
        Ok(val) => Some(val),
        Err(e) => {
            error!("Error getting value from byte slice {}", e);
            None
        }
    }
}
