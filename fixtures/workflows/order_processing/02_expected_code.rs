// Expected generated code for Order aggregate

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Order status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Created,
    Processing,
    Paid,
    Placed,
    Shipped,
    Delivered,
    Cancelled,
}

/// Order item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderItem {
    pub product_id: String,
    pub name: String,
    pub quantity: u32,
    pub unit_price: f64,
}

impl OrderItem {
    pub fn total_price(&self) -> f64 {
        self.quantity as f64 * self.unit_price
    }
}

/// Order aggregate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
}

impl Order {
    /// Create a new order
    pub fn new(customer_id: String) -> Self {
        Self {
            order_id: uuid::Uuid::new_v4().to_string(),
            customer_id,
            items: Vec::new(),
            subtotal: 0.0,
            tax: 0.0,
            total: 0.0,
            status: OrderStatus::Created,
            created_at: Utc::now(),
        }
    }

    /// Add item to order
    pub fn add_item(&mut self, item: OrderItem) {
        self.items.push(item);
    }

    /// Calculate totals
    pub fn calculate_total(&mut self, tax_rate: f64) {
        self.subtotal = self.items.iter().map(|i| i.total_price()).sum();
        self.tax = self.subtotal * tax_rate;
        self.total = self.subtotal + self.tax;
    }

    /// Update status
    pub fn update_status(&mut self, status: OrderStatus) {
        self.status = status;
    }
}

/// Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreatedEvent {
    pub order_id: String,
    pub customer_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemAddedEvent {
    pub order_id: String,
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedEvent {
    pub order_id: String,
    pub total: f64,
    pub timestamp: DateTime<Utc>,
}
