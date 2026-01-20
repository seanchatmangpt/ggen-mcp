# verify_receipt Tool Example

## Overview

The `verify_receipt` MCP tool provides cryptographic verification of ggen generation receipts. It performs 7 comprehensive checks to ensure receipt integrity and validate that generated code matches the expected outputs.

## 7 Verification Checks

1. **Schema Version** - Validates receipt format and version compatibility
2. **Workspace Fingerprint** - Verifies receipt was generated in expected workspace
3. **Input File Hashes** - Validates all input files (config, ontologies, queries, templates) match recorded SHA-256 hashes
4. **Output File Hashes** - Validates all generated output files match recorded SHA-256 hashes
5. **Guard Verdicts** - Verifies guard execution results are present and valid
6. **Metadata Consistency** - Validates timestamp and compiler version metadata
7. **Receipt ID Verification** - Validates cryptographic receipt ID format

## Receipt Format (v1.0.0)

```json
{
  "version": "1.0.0",
  "id": "a1b2c3d4e5f6...",
  "timestamp": "2026-01-20T12:34:56Z",
  "workspace": {
    "fingerprint": "sha256_hash_of_workspace_root",
    "root": "/path/to/workspace"
  },
  "inputs": {
    "config": {
      "path": "ggen.toml",
      "hash": "sha256_hash"
    },
    "ontologies": [
      {
        "path": "ontology/mcp-domain.ttl",
        "hash": "sha256_hash"
      }
    ],
    "queries": [
      {
        "path": "queries/extract_tools.rq",
        "hash": "sha256_hash"
      }
    ],
    "templates": [
      {
        "path": "templates/tool.rs.tera",
        "hash": "sha256_hash"
      }
    ]
  },
  "outputs": [
    {
      "path": "src/generated/tools.rs",
      "hash": "sha256_hash",
      "size_bytes": 12345
    }
  ],
  "guards": {
    "verdicts": [
      {
        "guard_name": "syntax_check",
        "verdict": "pass",
        "message": "All generated files compile successfully"
      },
      {
        "guard_name": "test_coverage",
        "verdict": "pass",
        "message": "Coverage > 80%"
      }
    ]
  },
  "metadata": {
    "timestamp": "2026-01-20T12:34:56Z",
    "compiler_version": "6.0.0",
    "generation_mode": "sync"
  }
}
```

## Usage Examples

### Example 1: Basic Receipt Verification

```bash
# MCP tool call
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": ".ggen/receipts/latest.json"
  }
}
```

**Response:**
```json
{
  "valid": true,
  "checks": [
    {
      "name": "Schema Version",
      "passed": true,
      "message": "Version 1.0.0 with valid ID"
    },
    {
      "name": "Input File Hashes",
      "passed": true,
      "message": "12 of 12 input files verified"
    },
    {
      "name": "Output File Hashes",
      "passed": true,
      "message": "8 of 8 output files verified"
    },
    {
      "name": "Guard Verdicts",
      "passed": true,
      "message": "6 passed, 0 failed"
    },
    {
      "name": "Metadata Consistency",
      "passed": true,
      "message": "Compiler v6.0.0"
    },
    {
      "name": "Receipt ID Verification",
      "passed": true,
      "message": "Valid SHA-256 ID: a1b2c3d4e5f6..."
    }
  ],
  "summary": "✅ Receipt valid (7 checks passed)",
  "receipt_info": {
    "id": "a1b2c3d4e5f6...",
    "timestamp": "2026-01-20T12:34:56Z",
    "compiler_version": "6.0.0",
    "input_count": 12,
    "output_count": 8
  }
}
```

### Example 2: Verify with Workspace Fingerprint

```bash
# MCP tool call with workspace verification
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": ".ggen/receipts/build-20260120.json",
    "workspace_root": "/home/user/ggen-mcp"
  }
}
```

**Response with Workspace Mismatch:**
```json
{
  "valid": false,
  "checks": [
    {
      "name": "Schema Version",
      "passed": true,
      "message": "Version 1.0.0 with valid ID"
    },
    {
      "name": "Workspace Fingerprint",
      "passed": false,
      "message": "Mismatch: expected 1a2b3c4d5e6f..., got 9f8e7d6c5b4a..."
    },
    {
      "name": "Input File Hashes",
      "passed": true,
      "message": "12 of 12 input files verified"
    },
    {
      "name": "Output File Hashes",
      "passed": true,
      "message": "8 of 8 output files verified"
    },
    {
      "name": "Guard Verdicts",
      "passed": true,
      "message": "6 passed, 0 failed"
    },
    {
      "name": "Metadata Consistency",
      "passed": true,
      "message": "Compiler v6.0.0"
    },
    {
      "name": "Receipt ID Verification",
      "passed": true,
      "message": "Valid SHA-256 ID: a1b2c3d4e5f6..."
    }
  ],
  "summary": "❌ Receipt invalid (1 of 7 checks failed)",
  "receipt_info": {
    "id": "a1b2c3d4e5f6...",
    "timestamp": "2026-01-20T12:34:56Z",
    "compiler_version": "6.0.0",
    "input_count": 12,
    "output_count": 8
  }
}
```

### Example 3: Hash Mismatch Detection

```bash
# After modifying an output file, verification fails
{
  "tool": "verify_receipt",
  "params": {
    "receipt_path": ".ggen/receipts/latest.json"
  }
}
```

**Response:**
```json
{
  "valid": false,
  "checks": [
    {
      "name": "Schema Version",
      "passed": true,
      "message": "Version 1.0.0 with valid ID"
    },
    {
      "name": "Input File Hashes",
      "passed": true,
      "message": "12 of 12 input files verified"
    },
    {
      "name": "Output File Hashes",
      "passed": false,
      "message": "1 hash mismatches: src/generated/tools.rs: Hash mismatch: expected 1a2b3c4d..., got 9f8e7d6c..."
    },
    {
      "name": "Guard Verdicts",
      "passed": true,
      "message": "6 passed, 0 failed"
    },
    {
      "name": "Metadata Consistency",
      "passed": true,
      "message": "Compiler v6.0.0"
    },
    {
      "name": "Receipt ID Verification",
      "passed": true,
      "message": "Valid SHA-256 ID: a1b2c3d4e5f6..."
    }
  ],
  "summary": "❌ Receipt invalid (1 of 7 checks failed)",
  "receipt_info": {
    "id": "a1b2c3d4e5f6...",
    "timestamp": "2026-01-20T12:34:56Z",
    "compiler_version": "6.0.0",
    "input_count": 12,
    "output_count": 8
  }
}
```

## Workflow Integration

### CI/CD Pipeline Usage

```yaml
# .github/workflows/verify-build.yml
- name: Generate code from ontology
  run: ggen sync --audit true

- name: Verify generation receipt
  run: |
    mcp_client call verify_receipt \
      --receipt-path .ggen/receipts/latest.json \
      --workspace-root $(pwd)

- name: Check verification result
  run: |
    if [ "$VERIFICATION_VALID" = "true" ]; then
      echo "✅ Receipt verification passed"
    else
      echo "❌ Receipt verification failed"
      exit 1
    fi
```

### Pre-Commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Verify latest receipt before committing
RECEIPT_PATH=".ggen/receipts/latest.json"

if [ -f "$RECEIPT_PATH" ]; then
  RESULT=$(mcp_client call verify_receipt --receipt-path "$RECEIPT_PATH")
  VALID=$(echo "$RESULT" | jq -r '.valid')

  if [ "$VALID" != "true" ]; then
    echo "❌ Receipt verification failed. Please regenerate code with 'ggen sync --audit true'"
    exit 1
  fi
fi
```

## Error Scenarios

### Missing Input Files

```json
{
  "valid": false,
  "checks": [
    {
      "name": "Input File Hashes",
      "passed": false,
      "message": "2 files missing: queries/extract_tools.rq, templates/tool.rs.tera"
    }
  ],
  "summary": "❌ Receipt invalid (1 of 7 checks failed)"
}
```

### Invalid Schema Version

```json
{
  "valid": false,
  "checks": [
    {
      "name": "Schema Version",
      "passed": false,
      "message": "Unsupported version: 0.9.0"
    }
  ],
  "summary": "❌ Receipt invalid (1 of 7 checks failed)"
}
```

### No Guard Verdicts

```json
{
  "valid": false,
  "checks": [
    {
      "name": "Guard Verdicts",
      "passed": false,
      "message": "No guard verdicts found"
    }
  ],
  "summary": "❌ Receipt invalid (1 of 7 checks failed)"
}
```

## Best Practices

1. **Always verify receipts after generation**
   ```bash
   ggen sync --audit true
   verify_receipt --receipt-path .ggen/receipts/latest.json
   ```

2. **Include workspace fingerprint in CI**
   ```bash
   verify_receipt \
     --receipt-path .ggen/receipts/latest.json \
     --workspace-root $(pwd)
   ```

3. **Archive receipts with version control**
   ```bash
   git add .ggen/receipts/
   git commit -m "chore: Add generation receipt for build $(date +%Y%m%d)"
   ```

4. **Verify receipts before deployment**
   ```bash
   # In production deployment script
   if ! verify_receipt --receipt-path .ggen/receipts/latest.json; then
     echo "❌ Receipt verification failed - aborting deployment"
     exit 1
   fi
   ```

## Receipt Storage Recommendations

1. **Timestamped receipts**: `.ggen/receipts/build-20260120-123456.json`
2. **Latest symlink**: `.ggen/receipts/latest.json` → most recent receipt
3. **Git tracking**: Add receipts to version control for audit trail
4. **Rotation policy**: Keep last 30 days of receipts (configurable)

## Security Considerations

1. **SHA-256 hashing**: All file hashes use cryptographic SHA-256
2. **Tampering detection**: Any modification to inputs/outputs detected
3. **Workspace isolation**: Fingerprint ensures receipt matches expected workspace
4. **Immutable receipts**: Receipt ID verification prevents receipt tampering

## Performance

- **Verification time**: O(n) where n = number of input + output files
- **Hash computation**: ~1ms per file (typical)
- **Receipt parsing**: <10ms (typical JSON size: 10-50KB)
- **Total verification**: <1 second for typical projects (20-50 files)

## Troubleshooting

### Issue: "Hash mismatch" on valid files

**Cause**: Line ending differences (CRLF vs LF) or formatting changes

**Solution**: Re-run `ggen sync --audit true` to regenerate receipt

### Issue: "Workspace fingerprint mismatch"

**Cause**: Receipt generated in different workspace directory

**Solution**: Omit `workspace_root` parameter or regenerate receipt in current workspace

### Issue: "Missing guard verdicts"

**Cause**: Receipt generated without guard execution

**Solution**: Ensure `ggen sync` runs with guard validation enabled

## See Also

- [ggen Sync Tool](./ggen_sync.md) - Generate receipts during sync
- [Receipt Format Specification](../docs/RECEIPT_FORMAT.md) - Detailed receipt schema
- [TPS Quality Gates](../docs/POKA_YOKE_IMPLEMENTATION.md) - Quality assurance patterns
