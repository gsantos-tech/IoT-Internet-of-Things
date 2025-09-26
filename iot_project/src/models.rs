use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateItem {
    pub nome: String,
}
