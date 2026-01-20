use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::value_objects::Money;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price: Money,
    pub sku: String,
}

impl Product {
    pub fn new(name: String, price: Money, sku: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            price,
            sku,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
}
