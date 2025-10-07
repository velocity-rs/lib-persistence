use std::collections::BTreeMap;

use jsonschema::ValidationError;


pub fn get_valerr_map(val_errors: Box<dyn Iterator<Item = ValidationError<'_>> + Send + Sync>) -> BTreeMap<String, String> {

    let mut err_map = BTreeMap::<String, String>::new();

    for val_err in val_errors {
        let instance_path = match val_err.instance_path.to_string().len() {
            0 => "/".to_string(),
            _ => val_err.instance_path.to_string(),
        };
        err_map.insert(val_err.to_string(), instance_path);
    }
    return err_map.clone();
}