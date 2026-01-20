# ggen Sync Report
**Workspace**: 7f83b165
**Timestamp**: 2026-01-20T16:30:00Z
**Mode**: apply
**Status**: ✅ PASS

## Inputs Discovered
- Config: ggen.toml (31 rules)
- Ontologies: ontology/mcp-domain.ttl (42KB)
- Queries: 14 files (queries/*.rq)
- Templates: 21 files (templates/*.rs.tera)

## Guard Verdicts
✅ G1: Path safety (no traversal)
✅ G2: Output overlap (no conflicts)
✅ G3: Template compilation (21/21 valid)
✅ G4: Turtle parse (valid RDF)
✅ G5: SPARQL execution (14/14 pass)
✅ G6: Determinism (hash stable)
✅ G7: Size/time bounds (within limits)

## Changes
- Files added: 13
- Files modified: 0
- Files deleted: 0
- Total LOC: 3,420

## Validation
✅ Rust: 8 files (0 errors)
✅ TypeScript: 3 files (0 errors)
✅ YAML: 2 files (0 errors)

## Performance
- Discovery: 45ms
- SPARQL: 230ms (14 queries, 80% cache hit)
- Render: 180ms (21 templates, 60% cache hit)
- Validate: 95ms
- **Total**: 550ms

## Receipts
- Report: ./ggen.out/reports/20260120-163000.md
- Receipt: ./ggen.out/receipts/sync-20260120-163000.json
- Diff: ./ggen.out/diffs/sync-20260120-163000.patch
