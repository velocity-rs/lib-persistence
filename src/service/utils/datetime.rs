use log::error;
use mongodb::bson::Document;

pub fn update_one_ts(doc: &Document) -> Document {
    let mut updated_doc = update_doc("created_at", doc);
    updated_doc = update_doc("updated_at", &updated_doc);
    updated_doc
}

#[expect(unused)]
pub fn update_many_ts(docs: Vec<Document>) -> Vec<Document> {
    let updated_docs = docs.iter().map(|d| update_one_ts(d)).collect();
    updated_docs
}

fn update_doc(key: &str, doc: &Document) -> Document {
    let mut updated_doc = Document::from(doc.to_owned());
    match doc.get(key) {
        Some(bson_dt) => match bson_dt.as_datetime() {
            Some(dt) => {
                updated_doc.insert(key, dt.to_string());
            }
            None => {
                error!("Not a datetime field. Cannot update");
            }
        },
        None => {}
    }
    updated_doc
}

/*
    let updated_results: Document> = results.iter()
            .map(|v: &Document| {

            }).collect();
}*/
