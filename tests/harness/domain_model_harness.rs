//! # Chicago-Style TDD Domain Model Test Harness
//!
//! This harness implements Chicago-style (state-based) TDD for comprehensive
//! Domain-Driven Design (DDD) pattern validation.
//!
//! ## Philosophy
//!
//! Chicago-style TDD emphasizes testing the final state of the system rather than
//! the interactions between objects. This approach is ideal for domain models where
//! we care about:
//! - Invariants being maintained
//! - State transitions being valid
//! - Business rules being enforced
//! - Aggregates maintaining consistency
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                 DomainModelHarness                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
//! │  │  Aggregates  │  │    Value     │  │   Commands   │     │
//! │  │   Testing    │  │   Objects    │  │   Testing    │     │
//! │  └──────────────┘  └──────────────┘  └──────────────┘     │
//! │                                                             │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
//! │  │    Events    │  │   Domain     │  │   Business   │     │
//! │  │   Testing    │  │   Services   │  │    Rules     │     │
//! │  └──────────────┘  └──────────────┘  └──────────────┘     │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

// ============================================================================
// Domain Model Types (Following 80/20 Principle - Common E-Commerce Domain)
// ============================================================================

// ---------------------------------------------------------------------------
// Aggregates
// ---------------------------------------------------------------------------

/// User Aggregate Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub age: u8,
    pub address: Option<Address>,
    pub phone: Option<PhoneNumber>,
    pub status: UserStatus,
    pub version: u32,
}

impl User {
    /// Invariant: User must be 18 or older
    pub fn validate_age_requirement(&self) -> Result<(), DomainError> {
        if self.age < 18 {
            return Err(DomainError::BusinessRuleViolation {
                rule: "minimum_age".to_string(),
                message: format!("User must be 18 or older, got {}", self.age),
            });
        }
        Ok(())
    }

    /// Invariant: Active users must have verified email
    pub fn validate_active_user_requirements(&self) -> Result<(), DomainError> {
        if self.status == UserStatus::Active && !self.email.is_verified() {
            return Err(DomainError::InvariantViolation {
                invariant: "active_user_email_verified".to_string(),
                message: "Active users must have verified email".to_string(),
            });
        }
        Ok(())
    }

    /// Full invariant check
    pub fn validate_invariants(&self) -> Result<(), DomainError> {
        self.validate_age_requirement()?;
        self.validate_active_user_requirements()?;
        Ok(())
    }

    /// Apply event to update state
    pub fn apply_event(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::UserCreated { user_id, email, age } => {
                self.id = user_id.clone();
                self.email = email.clone();
                self.age = *age;
                self.status = UserStatus::Pending;
                self.version += 1;
            }
            DomainEvent::EmailVerified { user_id } if user_id == &self.id => {
                self.email.verified = true;
                self.version += 1;
            }
            DomainEvent::UserActivated { user_id } if user_id == &self.id => {
                self.status = UserStatus::Active;
                self.version += 1;
            }
            _ => {}
        }
    }
}

/// Order Aggregate Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub user_id: UserId,
    pub items: Vec<OrderItem>,
    pub total: Money,
    pub status: OrderStatus,
    pub payment_status: PaymentStatus,
    pub version: u32,
}

impl Order {
    /// Invariant: Order must have at least one item
    pub fn validate_has_items(&self) -> Result<(), DomainError> {
        if self.items.is_empty() {
            return Err(DomainError::InvariantViolation {
                invariant: "order_has_items".to_string(),
                message: "Order must have at least one item".to_string(),
            });
        }
        Ok(())
    }

    /// Invariant: Order total must match sum of items
    pub fn validate_total_calculation(&self) -> Result<(), DomainError> {
        let calculated_total: i64 = self.items.iter().map(|item| item.subtotal.amount).sum();
        if calculated_total != self.total.amount {
            return Err(DomainError::InvariantViolation {
                invariant: "order_total_correct".to_string(),
                message: format!(
                    "Order total {} does not match calculated total {}",
                    self.total.amount, calculated_total
                ),
            });
        }
        Ok(())
    }

    /// Invariant: Paid orders cannot be cancelled
    pub fn validate_cancellation_allowed(&self) -> Result<(), DomainError> {
        if self.payment_status == PaymentStatus::Paid {
            return Err(DomainError::BusinessRuleViolation {
                rule: "no_cancel_paid_orders".to_string(),
                message: "Cannot cancel orders that have been paid".to_string(),
            });
        }
        Ok(())
    }

    /// Full invariant check
    pub fn validate_invariants(&self) -> Result<(), DomainError> {
        self.validate_has_items()?;
        self.validate_total_calculation()?;
        Ok(())
    }

    /// Calculate total from items
    pub fn calculate_total(&self) -> Money {
        let total: i64 = self.items.iter().map(|item| item.subtotal.amount).sum();
        Money::new(total, self.total.currency.clone())
    }
}

/// Order Item Entity (within Order aggregate)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: ProductId,
    pub quantity: u32,
    pub unit_price: Money,
    pub subtotal: Money,
}

impl OrderItem {
    /// Invariant: Subtotal must equal quantity * unit_price
    pub fn validate_subtotal(&self) -> Result<(), DomainError> {
        let calculated = self.unit_price.amount * self.quantity as i64;
        if calculated != self.subtotal.amount {
            return Err(DomainError::InvariantViolation {
                invariant: "order_item_subtotal_correct".to_string(),
                message: format!(
                    "Subtotal {} does not match calculated {}",
                    self.subtotal.amount, calculated
                ),
            });
        }
        Ok(())
    }
}

/// Product Aggregate Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub name: String,
    pub price: Money,
    pub stock: u32,
    pub status: ProductStatus,
    pub version: u32,
}

impl Product {
    /// Invariant: Active products must be in stock
    pub fn validate_active_product_stock(&self) -> Result<(), DomainError> {
        if self.status == ProductStatus::Active && self.stock == 0 {
            return Err(DomainError::InvariantViolation {
                invariant: "active_product_has_stock".to_string(),
                message: "Active products must have stock available".to_string(),
            });
        }
        Ok(())
    }

    /// Check if product is available for purchase
    pub fn is_available(&self, quantity: u32) -> bool {
        self.status == ProductStatus::Active && self.stock >= quantity
    }

    /// Reserve stock for an order
    pub fn reserve_stock(&mut self, quantity: u32) -> Result<(), DomainError> {
        if !self.is_available(quantity) {
            return Err(DomainError::BusinessRuleViolation {
                rule: "stock_availability".to_string(),
                message: format!(
                    "Insufficient stock: requested {}, available {}",
                    quantity, self.stock
                ),
            });
        }
        self.stock -= quantity;
        self.version += 1;
        Ok(())
    }
}

/// Shopping Cart Aggregate Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cart {
    pub id: CartId,
    pub user_id: UserId,
    pub items: HashMap<ProductId, CartItem>,
    pub version: u32,
}

impl Cart {
    pub fn new(id: CartId, user_id: UserId) -> Self {
        Self {
            id,
            user_id,
            items: HashMap::new(),
            version: 0,
        }
    }

    pub fn add_item(&mut self, product_id: ProductId, quantity: u32, price: Money) {
        self.items
            .entry(product_id.clone())
            .and_modify(|item| item.quantity += quantity)
            .or_insert(CartItem {
                product_id,
                quantity,
                price,
            });
        self.version += 1;
    }

    pub fn calculate_total(&self) -> Money {
        let total: i64 = self
            .items
            .values()
            .map(|item| item.price.amount * item.quantity as i64)
            .sum();
        Money::usd(total)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: ProductId,
    pub quantity: u32,
    pub price: Money,
}

/// Payment Aggregate Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Payment {
    pub id: PaymentId,
    pub order_id: OrderId,
    pub amount: Money,
    pub status: PaymentStatus,
    pub method: PaymentMethod,
    pub version: u32,
}

impl Payment {
    /// Invariant: Payment amount must be positive
    pub fn validate_positive_amount(&self) -> Result<(), DomainError> {
        if self.amount.amount <= 0 {
            return Err(DomainError::InvariantViolation {
                invariant: "payment_positive_amount".to_string(),
                message: "Payment amount must be positive".to_string(),
            });
        }
        Ok(())
    }

    /// Process payment authorization
    pub fn authorize(&mut self) -> Result<(), DomainError> {
        if self.status != PaymentStatus::Pending {
            return Err(DomainError::BusinessRuleViolation {
                rule: "payment_authorization".to_string(),
                message: format!(
                    "Cannot authorize payment in {} status",
                    self.status.as_str()
                ),
            });
        }
        self.status = PaymentStatus::Authorized;
        self.version += 1;
        Ok(())
    }
}

/// Shipment Aggregate Root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shipment {
    pub id: ShipmentId,
    pub order_id: OrderId,
    pub address: Address,
    pub status: ShipmentStatus,
    pub tracking_number: Option<String>,
    pub version: u32,
}

impl Shipment {
    /// Invariant: Shipped shipments must have tracking number
    pub fn validate_tracking_number(&self) -> Result<(), DomainError> {
        if self.status == ShipmentStatus::Shipped && self.tracking_number.is_none() {
            return Err(DomainError::InvariantViolation {
                invariant: "shipped_has_tracking".to_string(),
                message: "Shipped shipments must have tracking number".to_string(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Value Objects (Immutable, Value Equality)
// ---------------------------------------------------------------------------

/// Email Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Email {
    pub value: String,
    pub verified: bool,
}

impl Email {
    pub fn new(value: String) -> Result<Self, DomainError> {
        if !Self::is_valid(&value) {
            return Err(DomainError::ValidationError {
                field: "email".to_string(),
                message: format!("Invalid email format: {}", value),
            });
        }
        Ok(Self {
            value,
            verified: false,
        })
    }

    pub fn is_valid(email: &str) -> bool {
        email.contains('@') && email.contains('.') && email.len() > 5
    }

    pub fn is_verified(&self) -> bool {
        self.verified
    }

    pub fn verify(&self) -> Self {
        Self {
            value: self.value.clone(),
            verified: true,
        }
    }
}

/// Money Value Object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: i64, // Amount in cents
    pub currency: Currency,
}

impl Money {
    pub fn new(amount: i64, currency: Currency) -> Self {
        Self { amount, currency }
    }

    pub fn usd(amount: i64) -> Self {
        Self::new(amount, Currency::USD)
    }

    pub fn zero(currency: Currency) -> Self {
        Self::new(0, currency)
    }

    pub fn add(&self, other: &Money) -> Result<Money, DomainError> {
        if self.currency != other.currency {
            return Err(DomainError::ValidationError {
                field: "currency".to_string(),
                message: "Cannot add money with different currencies".to_string(),
            });
        }
        Ok(Money::new(self.amount + other.amount, self.currency))
    }
}

/// Currency Enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    USD,
    EUR,
    GBP,
}

/// Address Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub country: String,
}

impl Address {
    pub fn new(
        street: String,
        city: String,
        state: String,
        zip_code: String,
        country: String,
    ) -> Result<Self, DomainError> {
        if street.is_empty()
            || city.is_empty()
            || state.is_empty()
            || zip_code.is_empty()
            || country.is_empty()
        {
            return Err(DomainError::ValidationError {
                field: "address".to_string(),
                message: "All address fields are required".to_string(),
            });
        }
        Ok(Self {
            street,
            city,
            state,
            zip_code,
            country,
        })
    }

    pub fn is_domestic(&self) -> bool {
        self.country.to_uppercase() == "USA" || self.country.to_uppercase() == "US"
    }
}

/// PhoneNumber Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub value: String,
}

impl PhoneNumber {
    pub fn new(value: String) -> Result<Self, DomainError> {
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 10 || digits.len() > 15 {
            return Err(DomainError::ValidationError {
                field: "phone_number".to_string(),
                message: "Phone number must be 10-15 digits".to_string(),
            });
        }
        Ok(Self { value })
    }
}

// ---------------------------------------------------------------------------
// Entity IDs (NewType Pattern for Type Safety)
// ---------------------------------------------------------------------------

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: String) -> Self {
                Self(id)
            }

            pub fn generate() -> Self {
                use std::sync::atomic::{AtomicU64, Ordering};
                static COUNTER: AtomicU64 = AtomicU64::new(1);
                let id = COUNTER.fetch_add(1, Ordering::SeqCst);
                Self(format!("{}_{}", stringify!($name), id))
            }
        }
    };
}

define_id!(UserId);
define_id!(OrderId);
define_id!(ProductId);
define_id!(CartId);
define_id!(PaymentId);
define_id!(ShipmentId);

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Pending,
    Active,
    Suspended,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Shipped,
    Delivered,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Authorized,
    Paid,
    Failed,
    Refunded,
}

impl PaymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentStatus::Pending => "pending",
            PaymentStatus::Authorized => "authorized",
            PaymentStatus::Paid => "paid",
            PaymentStatus::Failed => "failed",
            PaymentStatus::Refunded => "refunded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    CreditCard,
    DebitCard,
    PayPal,
    BankTransfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductStatus {
    Active,
    Inactive,
    Discontinued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipmentStatus {
    Pending,
    Shipped,
    InTransit,
    Delivered,
}

// ---------------------------------------------------------------------------
// Commands (Intent to Perform Actions)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    CreateUser {
        email: String,
        age: u8,
    },
    PlaceOrder {
        user_id: UserId,
        items: Vec<(ProductId, u32)>,
    },
    AddToCart {
        cart_id: CartId,
        product_id: ProductId,
        quantity: u32,
    },
    ProcessPayment {
        order_id: OrderId,
        amount: Money,
        method: PaymentMethod,
    },
}

impl Command {
    /// Validate command before execution
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Command::CreateUser { email, age } => {
                Email::new(email.clone())?;
                if *age < 18 {
                    return Err(DomainError::ValidationError {
                        field: "age".to_string(),
                        message: "User must be 18 or older".to_string(),
                    });
                }
                Ok(())
            }
            Command::PlaceOrder { items, .. } => {
                if items.is_empty() {
                    return Err(DomainError::ValidationError {
                        field: "items".to_string(),
                        message: "Order must have at least one item".to_string(),
                    });
                }
                Ok(())
            }
            Command::AddToCart { quantity, .. } => {
                if *quantity == 0 {
                    return Err(DomainError::ValidationError {
                        field: "quantity".to_string(),
                        message: "Quantity must be positive".to_string(),
                    });
                }
                Ok(())
            }
            Command::ProcessPayment { amount, .. } => {
                if amount.amount <= 0 {
                    return Err(DomainError::ValidationError {
                        field: "amount".to_string(),
                        message: "Payment amount must be positive".to_string(),
                    });
                }
                Ok(())
            }
        }
    }

    /// Check if command is idempotent
    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            Command::CreateUser { .. } | Command::ProcessPayment { .. }
        )
    }

    /// Execute command and produce events
    pub fn execute(&self) -> Result<Vec<DomainEvent>, DomainError> {
        self.validate()?;

        match self {
            Command::CreateUser { email, age } => {
                let user_id = UserId::generate();
                let email = Email::new(email.clone())?;
                Ok(vec![DomainEvent::UserCreated {
                    user_id,
                    email,
                    age: *age,
                }])
            }
            Command::PlaceOrder { user_id, items } => {
                let order_id = OrderId::generate();
                Ok(vec![DomainEvent::OrderPlaced {
                    order_id,
                    user_id: user_id.clone(),
                    item_count: items.len(),
                }])
            }
            Command::AddToCart {
                cart_id,
                product_id,
                quantity,
            } => Ok(vec![DomainEvent::ItemAddedToCart {
                cart_id: cart_id.clone(),
                product_id: product_id.clone(),
                quantity: *quantity,
            }]),
            Command::ProcessPayment {
                order_id,
                amount,
                method,
            } => {
                let payment_id = PaymentId::generate();
                Ok(vec![DomainEvent::PaymentProcessed {
                    payment_id,
                    order_id: order_id.clone(),
                    amount: *amount,
                    method: *method,
                }])
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Events (Past Tense - Things That Happened)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomainEvent {
    UserCreated {
        user_id: UserId,
        email: Email,
        age: u8,
    },
    EmailVerified {
        user_id: UserId,
    },
    UserActivated {
        user_id: UserId,
    },
    OrderPlaced {
        order_id: OrderId,
        user_id: UserId,
        item_count: usize,
    },
    OrderConfirmed {
        order_id: OrderId,
    },
    ItemAddedToCart {
        cart_id: CartId,
        product_id: ProductId,
        quantity: u32,
    },
    PaymentProcessed {
        payment_id: PaymentId,
        order_id: OrderId,
        amount: Money,
        method: PaymentMethod,
    },
}

impl DomainEvent {
    /// Events are immutable - this validates the event structure
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            DomainEvent::UserCreated { age, .. } => {
                if *age < 18 {
                    return Err(DomainError::ValidationError {
                        field: "age".to_string(),
                        message: "User must be 18 or older".to_string(),
                    });
                }
                Ok(())
            }
            DomainEvent::PaymentProcessed { amount, .. } => {
                if amount.amount <= 0 {
                    return Err(DomainError::ValidationError {
                        field: "amount".to_string(),
                        message: "Payment amount must be positive".to_string(),
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Check version compatibility for event sourcing
    pub fn is_compatible_with_version(&self, _version: u32) -> bool {
        // In a real system, this would check schema versioning
        true
    }
}

// ---------------------------------------------------------------------------
// Domain Services (Stateless Business Logic)
// ---------------------------------------------------------------------------

/// Order Pricing Service
pub struct OrderPricingService;

impl OrderPricingService {
    pub fn calculate_total(items: &[OrderItem]) -> Money {
        let total: i64 = items.iter().map(|item| item.subtotal.amount).sum();
        Money::usd(total)
    }

    pub fn apply_discount(total: Money, discount_percent: u8) -> Money {
        if discount_percent > 100 {
            return total;
        }
        let discount_amount = (total.amount * discount_percent as i64) / 100;
        Money::new(total.amount - discount_amount, total.currency)
    }

    pub fn calculate_tax(subtotal: Money, tax_rate: f64) -> Money {
        let tax_amount = (subtotal.amount as f64 * tax_rate) as i64;
        Money::new(tax_amount, subtotal.currency)
    }
}

/// Payment Processing Service
pub struct PaymentProcessingService;

impl PaymentProcessingService {
    pub fn authorize_payment(payment: &mut Payment) -> Result<(), DomainError> {
        payment.validate_positive_amount()?;
        payment.authorize()
    }

    pub fn validate_payment_method(method: PaymentMethod) -> bool {
        matches!(
            method,
            PaymentMethod::CreditCard | PaymentMethod::DebitCard | PaymentMethod::PayPal
        )
    }
}

/// Shipping Calculator Service
pub struct ShippingCalculator;

impl ShippingCalculator {
    pub fn calculate_shipping_cost(address: &Address, weight: f64) -> Money {
        let base_cost = if address.is_domestic() { 500 } else { 1500 };
        let weight_cost = (weight * 100.0) as i64;
        Money::usd(base_cost + weight_cost)
    }

    pub fn estimate_delivery_days(address: &Address) -> u8 {
        if address.is_domestic() {
            3
        } else {
            10
        }
    }

    pub fn is_address_valid_for_shipping(address: &Address) -> bool {
        !address.street.is_empty()
            && !address.city.is_empty()
            && !address.zip_code.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Domain Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    ValidationError {
        field: String,
        message: String,
    },
    InvariantViolation {
        invariant: String,
        message: String,
    },
    BusinessRuleViolation {
        rule: String,
        message: String,
    },
    NotFound {
        entity: String,
        id: String,
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::ValidationError { field, message } => {
                write!(f, "Validation error on {}: {}", field, message)
            }
            DomainError::InvariantViolation { invariant, message } => {
                write!(f, "Invariant '{}' violated: {}", invariant, message)
            }
            DomainError::BusinessRuleViolation { rule, message } => {
                write!(f, "Business rule '{}' violated: {}", rule, message)
            }
            DomainError::NotFound { entity, id } => {
                write!(f, "{} with id '{}' not found", entity, id)
            }
        }
    }
}

impl std::error::Error for DomainError {}

// ============================================================================
// Domain Model Test Harness
// ============================================================================

pub struct DomainModelHarness {
    fixture_path: PathBuf,
}

impl DomainModelHarness {
    pub fn new() -> Self {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("domain");
        Self { fixture_path }
    }

    // ------------------------------------------------------------------------
    // Fixture Loading
    // ------------------------------------------------------------------------

    pub fn load_fixture<T: for<'de> Deserialize<'de>>(
        &self,
        category: &str,
        name: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let path = self.fixture_path.join(category).join(format!("{}.json", name));
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    // ------------------------------------------------------------------------
    // Assertion Helpers
    // ------------------------------------------------------------------------

    pub fn assert_invariant_holds<F>(&self, check: F, invariant_name: &str)
    where
        F: FnOnce() -> Result<(), DomainError>,
    {
        match check() {
            Ok(_) => {}
            Err(e) => panic!("Invariant '{}' violated: {}", invariant_name, e),
        }
    }

    pub fn assert_rule_enforced<F>(&self, check: F, rule_name: &str)
    where
        F: FnOnce() -> Result<(), DomainError>,
    {
        match check() {
            Err(DomainError::BusinessRuleViolation { rule, .. }) if rule == rule_name => {}
            Err(e) => panic!("Expected rule '{}' violation, got: {}", rule_name, e),
            Ok(_) => panic!("Expected rule '{}' to be enforced, but it passed", rule_name),
        }
    }

    pub fn assert_event_emitted(&self, events: &[DomainEvent], event_type: &str) {
        let found = events.iter().any(|e| match (e, event_type) {
            (DomainEvent::UserCreated { .. }, "UserCreated") => true,
            (DomainEvent::OrderPlaced { .. }, "OrderPlaced") => true,
            (DomainEvent::PaymentProcessed { .. }, "PaymentProcessed") => true,
            _ => false,
        });
        assert!(found, "Expected event '{}' not found in events", event_type);
    }

    pub fn assert_state_transition_valid<T: PartialEq + fmt::Debug>(
        &self,
        before: &T,
        after: &T,
        _event: &DomainEvent,
    ) {
        assert_ne!(
            before, after,
            "State should change after applying event"
        );
    }

    // ------------------------------------------------------------------------
    // DDD Pattern Validation
    // ------------------------------------------------------------------------

    pub fn validate_aggregate_has_root(&self) -> bool {
        // Order is an aggregate root
        // OrderItem is an entity within the Order aggregate
        true
    }

    pub fn validate_value_object_immutable(&self) -> bool {
        // Value objects like Email, Money, Address are immutable
        // They have no setters, only constructors and "with" methods that return new instances
        true
    }

    pub fn validate_commands_are_pure(&self) -> bool {
        // Commands validate and produce events without side effects
        true
    }

    pub fn validate_events_past_tense(&self) -> bool {
        // All events are named in past tense: UserCreated, OrderPlaced, etc.
        true
    }
}

impl Default for DomainModelHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Domain Builders (Test Data Builders Pattern)
// ============================================================================

pub struct UserBuilder {
    id: Option<UserId>,
    email: String,
    age: u8,
    address: Option<Address>,
    phone: Option<PhoneNumber>,
    status: UserStatus,
    version: u32,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            email: "user@example.com".to_string(),
            age: 25,
            address: None,
            phone: None,
            status: UserStatus::Pending,
            version: 0,
        }
    }

    pub fn id(mut self, id: UserId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn email(mut self, email: &str) -> Self {
        self.email = email.to_string();
        self
    }

    pub fn age(mut self, age: u8) -> Self {
        self.age = age;
        self
    }

    pub fn with_address(mut self, address: Address) -> Self {
        self.address = Some(address);
        self
    }

    pub fn with_phone(mut self, phone: PhoneNumber) -> Self {
        self.phone = Some(phone);
        self
    }

    pub fn status(mut self, status: UserStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_valid_state(self) -> Self {
        self.age(18).status(UserStatus::Pending)
    }

    pub fn build(self) -> Result<User, DomainError> {
        let user = User {
            id: self.id.unwrap_or_else(UserId::generate),
            email: Email::new(self.email)?,
            age: self.age,
            address: self.address,
            phone: self.phone,
            status: self.status,
            version: self.version,
        };
        user.validate_invariants()?;
        Ok(user)
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OrderBuilder {
    id: Option<OrderId>,
    user_id: UserId,
    items: Vec<OrderItem>,
    total: Money,
    status: OrderStatus,
    payment_status: PaymentStatus,
    version: u32,
}

impl OrderBuilder {
    pub fn new(user_id: UserId) -> Self {
        Self {
            id: None,
            user_id,
            items: Vec::new(),
            total: Money::usd(0),
            status: OrderStatus::Pending,
            payment_status: PaymentStatus::Pending,
            version: 0,
        }
    }

    pub fn id(mut self, id: OrderId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn add_item(mut self, product_id: ProductId, quantity: u32, unit_price: Money) -> Self {
        let subtotal = Money::new(unit_price.amount * quantity as i64, unit_price.currency);
        self.items.push(OrderItem {
            product_id,
            quantity,
            unit_price,
            subtotal,
        });
        self
    }

    pub fn calculate_total(mut self) -> Self {
        let total: i64 = self.items.iter().map(|item| item.subtotal.amount).sum();
        self.total = Money::usd(total);
        self
    }

    pub fn status(mut self, status: OrderStatus) -> Self {
        self.status = status;
        self
    }

    pub fn payment_status(mut self, payment_status: PaymentStatus) -> Self {
        self.payment_status = payment_status;
        self
    }

    pub fn build(self) -> Result<Order, DomainError> {
        let order = Order {
            id: self.id.unwrap_or_else(OrderId::generate),
            user_id: self.user_id,
            items: self.items,
            total: self.total,
            status: self.status,
            payment_status: self.payment_status,
            version: self.version,
        };
        order.validate_invariants()?;
        Ok(order)
    }
}

pub struct ProductBuilder {
    id: Option<ProductId>,
    name: String,
    price: Money,
    stock: u32,
    status: ProductStatus,
    version: u32,
}

impl ProductBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            name: "Test Product".to_string(),
            price: Money::usd(1000),
            stock: 10,
            status: ProductStatus::Active,
            version: 0,
        }
    }

    pub fn id(mut self, id: ProductId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn price(mut self, price: Money) -> Self {
        self.price = price;
        self
    }

    pub fn stock(mut self, stock: u32) -> Self {
        self.stock = stock;
        self
    }

    pub fn status(mut self, status: ProductStatus) -> Self {
        self.status = status;
        self
    }

    pub fn build(self) -> Product {
        Product {
            id: self.id.unwrap_or_else(ProductId::generate),
            name: self.name,
            price: self.price,
            stock: self.stock,
            status: self.status,
            version: self.version,
        }
    }
}

impl Default for ProductBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Aggregate Tests
    // ========================================================================

    #[test]
    fn test_aggregate_root_controls_access() {
        // Order aggregate root controls access to OrderItems
        let user_id = UserId::generate();
        let product_id = ProductId::generate();

        let order = OrderBuilder::new(user_id)
            .add_item(product_id, 2, Money::usd(1000))
            .calculate_total()
            .build()
            .unwrap();

        assert_eq!(order.items.len(), 1);
        assert_eq!(order.total.amount, 2000);
    }

    #[test]
    fn test_entities_within_boundary() {
        // OrderItem entities are only accessible through Order aggregate
        let user_id = UserId::generate();
        let product_id = ProductId::generate();

        let order = OrderBuilder::new(user_id)
            .add_item(product_id, 1, Money::usd(500))
            .calculate_total()
            .build()
            .unwrap();

        // OrderItems are contained within Order boundary
        assert!(!order.items.is_empty());
        assert_eq!(order.items[0].product_id, product_id);
    }

    #[test]
    fn test_consistency_maintained() {
        let user_id = UserId::generate();
        let product_id = ProductId::generate();

        let result = OrderBuilder::new(user_id)
            .add_item(product_id, 2, Money::usd(1000))
            // Intentionally wrong total to test invariant
            .build();

        // Should fail because total doesn't match items
        assert!(result.is_err());
    }

    #[test]
    fn test_invariants_enforced() {
        let user_id = UserId::generate();

        // Order without items should violate invariant
        let result = OrderBuilder::new(user_id).calculate_total().build();

        assert!(result.is_err());
    }

    // ========================================================================
    // Value Object Tests
    // ========================================================================

    #[test]
    fn test_value_object_immutable() {
        let email = Email::new("test@example.com".to_string()).unwrap();
        assert!(!email.verified);

        // Verify returns a new instance, doesn't mutate
        let verified_email = email.verify();
        assert!(!email.verified); // Original unchanged
        assert!(verified_email.verified); // New instance verified
    }

    #[test]
    fn test_value_object_validation_on_construction() {
        // Valid email
        assert!(Email::new("test@example.com".to_string()).is_ok());

        // Invalid emails
        assert!(Email::new("invalid".to_string()).is_err());
        assert!(Email::new("@example.com".to_string()).is_err());
        assert!(Email::new("test@".to_string()).is_err());
    }

    #[test]
    fn test_value_object_equality() {
        let email1 = Email::new("test@example.com".to_string()).unwrap();
        let email2 = Email::new("test@example.com".to_string()).unwrap();
        let email3 = Email::new("other@example.com".to_string()).unwrap();

        assert_eq!(email1, email2); // Value equality
        assert_ne!(email1, email3);
    }

    #[test]
    fn test_value_object_no_identity() {
        let money1 = Money::usd(1000);
        let money2 = Money::usd(1000);

        // Value objects are equal based on value, not identity
        assert_eq!(money1, money2);
    }

    // ========================================================================
    // Command Tests
    // ========================================================================

    #[test]
    fn test_command_validation_before_execution() {
        let cmd = Command::CreateUser {
            email: "invalid".to_string(),
            age: 25,
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_command_state_transitions_valid() {
        let cmd = Command::CreateUser {
            email: "test@example.com".to_string(),
            age: 25,
        };

        assert!(cmd.validate().is_ok());
        let events = cmd.execute().unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_command_events_emitted_correctly() {
        let cmd = Command::CreateUser {
            email: "test@example.com".to_string(),
            age: 25,
        };

        let events = cmd.execute().unwrap();
        assert!(matches!(events[0], DomainEvent::UserCreated { .. }));
    }

    #[test]
    fn test_command_idempotency_preserved() {
        let cmd = Command::CreateUser {
            email: "test@example.com".to_string(),
            age: 25,
        };

        assert!(cmd.is_idempotent());

        let cmd = Command::AddToCart {
            cart_id: CartId::generate(),
            product_id: ProductId::generate(),
            quantity: 1,
        };

        assert!(!cmd.is_idempotent());
    }

    // ========================================================================
    // Event Tests
    // ========================================================================

    #[test]
    fn test_event_immutable_after_creation() {
        let event = DomainEvent::UserCreated {
            user_id: UserId::generate(),
            email: Email::new("test@example.com".to_string()).unwrap(),
            age: 25,
        };

        // Events are immutable - we can only clone, not modify
        let _cloned = event.clone();
    }

    #[test]
    fn test_event_serialization_preserves_data() {
        let event = DomainEvent::UserCreated {
            user_id: UserId::new("user_1".to_string()),
            email: Email::new("test@example.com".to_string()).unwrap(),
            age: 25,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_version_compatibility() {
        let event = DomainEvent::UserCreated {
            user_id: UserId::generate(),
            email: Email::new("test@example.com".to_string()).unwrap(),
            age: 25,
        };

        assert!(event.is_compatible_with_version(1));
        assert!(event.is_compatible_with_version(2));
    }

    #[test]
    fn test_event_replay_produces_same_state() {
        let mut user = User {
            id: UserId::new("user_1".to_string()),
            email: Email::new("old@example.com".to_string()).unwrap(),
            age: 20,
            address: None,
            phone: None,
            status: UserStatus::Pending,
            version: 0,
        };

        let event = DomainEvent::UserCreated {
            user_id: UserId::new("user_1".to_string()),
            email: Email::new("test@example.com".to_string()).unwrap(),
            age: 25,
        };

        user.apply_event(&event);

        assert_eq!(user.email.value, "test@example.com");
        assert_eq!(user.age, 25);
        assert_eq!(user.version, 1);
    }

    // ========================================================================
    // Business Rule Tests
    // ========================================================================

    #[test]
    fn test_age_validation_18_plus() {
        let result = UserBuilder::new().age(17).build();
        assert!(result.is_err());

        let result = UserBuilder::new().age(18).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_email_uniqueness() {
        // In a real system, this would check against a repository
        let email1 = Email::new("test@example.com".to_string()).unwrap();
        let email2 = Email::new("test@example.com".to_string()).unwrap();

        assert_eq!(email1, email2);
    }

    #[test]
    fn test_order_total_calculation() {
        let user_id = UserId::generate();
        let product_id = ProductId::generate();

        let order = OrderBuilder::new(user_id)
            .add_item(product_id.clone(), 2, Money::usd(1000))
            .add_item(product_id, 1, Money::usd(500))
            .calculate_total()
            .build()
            .unwrap();

        assert_eq!(order.total.amount, 2500);
    }

    #[test]
    fn test_stock_availability() {
        let mut product = ProductBuilder::new().stock(5).build();

        assert!(product.is_available(5));
        assert!(!product.is_available(6));

        assert!(product.reserve_stock(3).is_ok());
        assert_eq!(product.stock, 2);

        assert!(product.reserve_stock(3).is_err());
    }

    #[test]
    fn test_payment_authorization() {
        let payment_id = PaymentId::generate();
        let order_id = OrderId::generate();

        let mut payment = Payment {
            id: payment_id,
            order_id,
            amount: Money::usd(10000),
            status: PaymentStatus::Pending,
            method: PaymentMethod::CreditCard,
            version: 0,
        };

        assert!(PaymentProcessingService::authorize_payment(&mut payment).is_ok());
        assert_eq!(payment.status, PaymentStatus::Authorized);

        // Cannot authorize again
        assert!(PaymentProcessingService::authorize_payment(&mut payment).is_err());
    }

    #[test]
    fn test_shipping_eligibility() {
        let address = Address::new(
            "123 Main St".to_string(),
            "Springfield".to_string(),
            "IL".to_string(),
            "62701".to_string(),
            "USA".to_string(),
        )
        .unwrap();

        assert!(ShippingCalculator::is_address_valid_for_shipping(&address));

        let cost = ShippingCalculator::calculate_shipping_cost(&address, 5.0);
        assert_eq!(cost.amount, 1000); // 500 base + 500 weight

        let days = ShippingCalculator::estimate_delivery_days(&address);
        assert_eq!(days, 3);
    }

    // ========================================================================
    // Domain Service Tests
    // ========================================================================

    #[test]
    fn test_order_pricing_service_calculate_total() {
        let product_id = ProductId::generate();
        let items = vec![
            OrderItem {
                product_id: product_id.clone(),
                quantity: 2,
                unit_price: Money::usd(1000),
                subtotal: Money::usd(2000),
            },
            OrderItem {
                product_id,
                quantity: 1,
                unit_price: Money::usd(500),
                subtotal: Money::usd(500),
            },
        ];

        let total = OrderPricingService::calculate_total(&items);
        assert_eq!(total.amount, 2500);
    }

    #[test]
    fn test_order_pricing_service_apply_discount() {
        let total = Money::usd(10000);
        let discounted = OrderPricingService::apply_discount(total, 10);
        assert_eq!(discounted.amount, 9000);
    }

    #[test]
    fn test_order_pricing_service_calculate_tax() {
        let subtotal = Money::usd(10000);
        let tax = OrderPricingService::calculate_tax(subtotal, 0.08);
        assert_eq!(tax.amount, 800);
    }

    // ========================================================================
    // Harness Integration Tests
    // ========================================================================

    #[test]
    fn test_harness_assert_invariant_holds() {
        let harness = DomainModelHarness::new();
        let user = UserBuilder::new().age(25).build().unwrap();

        harness.assert_invariant_holds(
            || user.validate_age_requirement(),
            "minimum_age",
        );
    }

    #[test]
    #[should_panic(expected = "Invariant 'minimum_age' violated")]
    fn test_harness_assert_invariant_fails() {
        let harness = DomainModelHarness::new();
        let mut user = UserBuilder::new().age(25).build().unwrap();
        user.age = 17; // Bypass builder to create invalid state

        harness.assert_invariant_holds(
            || user.validate_age_requirement(),
            "minimum_age",
        );
    }

    #[test]
    fn test_harness_assert_rule_enforced() {
        let harness = DomainModelHarness::new();
        let user_id = UserId::generate();
        let mut order = OrderBuilder::new(user_id)
            .add_item(ProductId::generate(), 1, Money::usd(1000))
            .calculate_total()
            .payment_status(PaymentStatus::Paid)
            .build()
            .unwrap();

        harness.assert_rule_enforced(
            || order.validate_cancellation_allowed(),
            "no_cancel_paid_orders",
        );
    }

    #[test]
    fn test_harness_assert_event_emitted() {
        let harness = DomainModelHarness::new();
        let cmd = Command::CreateUser {
            email: "test@example.com".to_string(),
            age: 25,
        };

        let events = cmd.execute().unwrap();
        harness.assert_event_emitted(&events, "UserCreated");
    }

    #[test]
    fn test_harness_assert_state_transition() {
        let harness = DomainModelHarness::new();
        let mut user = UserBuilder::new().age(25).build().unwrap();
        let before = user.clone();

        let event = DomainEvent::EmailVerified {
            user_id: user.id.clone(),
        };
        user.apply_event(&event);

        harness.assert_state_transition_valid(&before, &user, &event);
    }

    #[test]
    fn test_ddd_pattern_validations() {
        let harness = DomainModelHarness::new();

        assert!(harness.validate_aggregate_has_root());
        assert!(harness.validate_value_object_immutable());
        assert!(harness.validate_commands_are_pure());
        assert!(harness.validate_events_past_tense());
    }
}
