// src/algorithm_library.rs

// Basic arithmetic operations
pub fn add(a: f64, b: f64) -> f64 {
    a + b
}

pub fn subtract(a: f64, b: f64) -> f64 {
    a - b
}

pub fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

pub fn divide(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        // Handle division by zero
        println!("Error: Division by zero!");
        return 0.0;
    }
    a / b
}
