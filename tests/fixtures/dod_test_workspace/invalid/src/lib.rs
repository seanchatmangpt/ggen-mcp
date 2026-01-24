//! Invalid test workspace with failing DoD checks

// Intentionally poorly formatted code
pub fn add(a:i32,b:i32)->i32{a+b}

// Failing test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 5); // Wrong!
    }
}

// TODO: Fix this code
// WARNING: Security issue - hardcoded secret
const API_KEY: &str = "sk-1234567890abcdef";
