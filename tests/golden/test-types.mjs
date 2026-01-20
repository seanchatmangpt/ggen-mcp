// Auto-generated Type Definitions
// Generated from: test-api.ttl

export interface Entity {
    type: string;
    name: string;
    description: string;
}

export interface User {
    user_id: string;
    name: string;
    email: string;
}

export interface Product {
    product_id: string;
    name: string;
    price: decimal;
}

export type EntityUnion = User | Product;
