#[cfg(test)]
mod tests {
    use chicago_tdd_tools::prelude::*;

    test!(test_ontology_validate, {
        // Arrange
        let ontology = spreadsheet_mcp::domain::aggregates::Ontology::new(
            "ont-test123".to_string(),
            "path/to/onto.ttl".to_string(),
        );

        // Act & Assert
        ontology.validate();
        assert!(!ontology.id.is_empty());
    });

    test!(test_receipt_validate, {
        // Arrange
        let receipt =
            spreadsheet_mcp::domain::aggregates::Receipt::new("receipt-123".to_string(), "hash1".to_string());

        // Act & Assert
        receipt.validate();
        assert_eq!(receipt.receipt_id, "receipt-123");
    });
}
