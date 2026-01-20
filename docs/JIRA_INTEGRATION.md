# Jira Integration Tool

## Overview

Create Jira tickets from spreadsheet rows via REST API v3. Batch creation with rate limiting, per-ticket error handling, dry-run validation.

## Tool: `create_jira_tickets_from_spreadsheet`

### Purpose
Reads spreadsheet data, maps columns to Jira fields, creates tickets via Jira Cloud/Server API.

### Parameters

```json
{
  "workbook_id": "my-spreadsheet.xlsx",
  "sheet_name": "Backlog",
  "jira_project_key": "PROJ",
  "jira_url": "https://your-domain.atlassian.net",
  "jira_auth": {
    "type": "bearer",
    "token": "your-api-token",
    "email": "user@example.com"
  },
  "column_mapping": {
    "summary_column": "A",
    "description_column": "B",
    "issue_type_column": "C",
    "priority_column": "D",
    "assignee_column": "E",
    "labels_column": "F",
    "epic_link_column": "G",
    "story_points_column": "H"
  },
  "dry_run": false,
  "start_row": 2,
  "max_tickets": 100
}
```

### Required Fields
- `workbook_id`: Workbook identifier
- `sheet_name`: Sheet containing ticket data
- `jira_project_key`: Jira project key (e.g., "PROJ")
- `jira_url`: Jira base URL (https://your-domain.atlassian.net)
- `jira_auth`: Authentication credentials (Bearer or Basic)
- `column_mapping.summary_column`: Ticket title/summary (required)
- `column_mapping.description_column`: Ticket description (required)
- `column_mapping.issue_type_column`: Issue type: Story/Bug/Task (required)

### Optional Fields
- `column_mapping.priority_column`: Priority (Highest/High/Medium/Low/Lowest)
- `column_mapping.assignee_column`: Assignee username
- `column_mapping.labels_column`: Comma-separated labels
- `column_mapping.epic_link_column`: Epic key (e.g., "EPIC-123")
- `column_mapping.story_points_column`: Story points (numeric)
- `dry_run`: Validate without creating (default: false)
- `start_row`: First data row (default: 2, skips header)
- `max_tickets`: Max tickets to create (default: 100, max: 100)

## Authentication

### Bearer Token (Personal Access Token)
```json
{
  "type": "bearer",
  "token": "your-personal-access-token"
}
```

### API Token (Atlassian Cloud)
```json
{
  "type": "bearer",
  "token": "your-api-token",
  "email": "user@example.com"
}
```

### Basic Auth
```json
{
  "type": "basic",
  "username": "admin",
  "password": "password"
}
```

## Spreadsheet Format

### Example Layout

| A (Summary) | B (Description) | C (Type) | D (Priority) | E (Assignee) | F (Labels) | G (Epic) |
|-------------|-----------------|----------|--------------|--------------|------------|----------|
| Fix login bug | Users can't login on mobile | Bug | High | john.doe | backend,auth | EPIC-1 |
| Add dark mode | Support dark theme in settings | Story | Medium | jane.smith | frontend,ui | EPIC-2 |
| API rate limiting | Implement rate limiter middleware | Task | High | dev.team | backend,api | |

### Column Mapping Rules
- **Column names**: A-ZZZ (uppercase letters only)
- **Summary**: Required, non-empty
- **Description**: Required, converted to Atlassian Document Format (ADF)
- **Issue Type**: Required (Story/Bug/Task/Epic/etc.)
- **Priority**: Optional (Highest/High/Medium/Low/Lowest)
- **Assignee**: Optional (Jira username)
- **Labels**: Optional (comma-separated, e.g., "backend,api,security")
- **Epic Link**: Optional (epic key, e.g., "EPIC-123")
- **Story Points**: Optional (numeric value)

## Response Format

```json
{
  "workbook_id": "spreadsheet.xlsx",
  "sheet_name": "Backlog",
  "dry_run": false,
  "total_rows_processed": 10,
  "tickets_created": 8,
  "tickets_failed": 2,
  "results": [
    {
      "row": 2,
      "success": true,
      "ticket_key": "PROJ-123",
      "ticket_url": "https://your-domain.atlassian.net/browse/PROJ-123",
      "summary": "Fix login bug",
      "error": null
    },
    {
      "row": 3,
      "success": false,
      "ticket_key": null,
      "ticket_url": null,
      "summary": "Invalid ticket",
      "error": "Jira API returned status 400: Invalid issue type"
    }
  ],
  "notes": [
    "2 ticket(s) failed to create (see results for details)"
  ]
}
```

## Safety Features

### Input Validation
- Non-empty required fields
- Valid HTTP(S) URLs
- Column names: A-ZZZ format
- Row ranges: 1-1,048,576
- Max batch size: 100 tickets

### Error Handling
- **Per-ticket errors**: Failures don't stop batch processing
- **Detailed error messages**: Jira API errors captured per row
- **Contextual errors**: Row numbers + summaries for failed tickets

### Rate Limiting
- 100ms delay between requests
- Prevents API abuse
- Configurable via `JIRA_RATE_LIMIT_DELAY_MS` constant

### Dry Run Mode
- `dry_run: true` validates parameters without creating tickets
- Tests spreadsheet parsing, column mapping, authentication headers
- Returns mock success results

## Workflow Example

### 1. Prepare Spreadsheet
Create spreadsheet with ticket data:
- Row 1: Headers
- Row 2+: Ticket data

### 2. Test with Dry Run
```json
{
  "workbook_id": "backlog.xlsx",
  "sheet_name": "Sprint 1",
  "jira_project_key": "PROJ",
  "jira_url": "https://company.atlassian.net",
  "jira_auth": {
    "type": "bearer",
    "token": "your-token",
    "email": "you@company.com"
  },
  "column_mapping": {
    "summary_column": "A",
    "description_column": "B",
    "issue_type_column": "C"
  },
  "dry_run": true,
  "start_row": 2,
  "max_tickets": 10
}
```

### 3. Create Tickets
Set `dry_run: false` and execute.

### 4. Review Results
- Check `tickets_created` vs `tickets_failed`
- Review `results` array for per-ticket status
- Inspect `error` field for failed tickets
- Use `ticket_url` to view created tickets in Jira

## Error Scenarios

### Authentication Failures
```json
{
  "error": "Jira API returned status 401: Unauthorized"
}
```
**Fix**: Verify token/credentials, check API token permissions.

### Invalid Issue Type
```json
{
  "error": "Jira API returned status 400: Issue type 'InvalidType' not found"
}
```
**Fix**: Use valid issue types (Story/Bug/Task/Epic). Check project configuration.

### Missing Required Fields
```json
{
  "error": "summary_column cannot be empty"
}
```
**Fix**: Ensure all required columns are mapped and non-empty.

### Sheet Not Found
```json
{
  "error": "failed to extract ticket data from sheet: Sheet 'Backlog' not found"
}
```
**Fix**: Verify sheet name matches workbook.

### Rate Limiting (Jira API)
```json
{
  "error": "Jira API returned status 429: Rate limit exceeded"
}
```
**Fix**: Tool has built-in 100ms delay. Jira may have stricter limits; reduce `max_tickets` or increase delay.

## Limitations

### Batch Size
- Max 100 tickets per request
- Use `start_row` + `max_tickets` to process in batches

### Field Support
Currently supports:
- Summary, Description, Issue Type (required)
- Priority, Assignee, Labels, Epic Link, Story Points (optional)

Not yet supported:
- Custom fields
- Attachments
- Watchers
- Components
- Fix versions
- Sprint assignment

### Jira API Version
- Targets Jira Cloud REST API v3
- Atlassian Document Format (ADF) for descriptions
- May require adjustments for Jira Server/Data Center

### Authentication
- Supports Bearer (PAT/API token) and Basic Auth
- Does not support OAuth 2.0

## Testing

### Unit Tests
```bash
cargo test --lib tools::jira_integration::tests
```

Tests include:
1. Parameter validation (success/failure)
2. Empty workbook ID rejection
3. Invalid URL rejection
4. Bearer token auth header generation
5. Basic auth header generation
6. Column mapping validation (success/failure)
7. Invalid column name rejection
8. Jira request building

### Integration Testing
Use dry-run mode to test without creating tickets:
```bash
# Test spreadsheet parsing + validation
{
  "dry_run": true,
  ...
}
```

### Mock Jira Server
For full integration tests, use tools like:
- `wiremock` (Rust)
- `mockito` (Rust)
- Docker container with mock Jira API

## Security Considerations

### Credentials
- **Never commit tokens/passwords to version control**
- Use environment variables or secret managers
- Rotate API tokens regularly

### Input Validation
- Path traversal prevention
- Column name sanitization (A-ZZZ only)
- URL validation (http/https)
- Max batch size enforcement

### Rate Limiting
- 100ms delay between requests
- Prevents API abuse
- Respects Jira API limits

### Error Disclosure
- Jira API errors returned verbatim
- May expose project/user existence
- Consider sanitizing errors in production

## Performance

### Batch Processing
- 100 tickets/request (max)
- ~10 tickets/second (with 100ms delay)
- 600 tickets/minute theoretical max

### Optimization Tips
1. Use `max_tickets` to limit batch size
2. Process large backlogs in multiple runs
3. Use `start_row` to resume after failures
4. Monitor Jira API rate limits

### Timeouts
- HTTP client timeout: 30 seconds per request
- Total batch timeout: depends on batch size (30s × ticket count)

## Examples

### Minimal Example (Required Fields Only)
```json
{
  "workbook_id": "tickets.xlsx",
  "sheet_name": "Sheet1",
  "jira_project_key": "TEST",
  "jira_url": "https://test.atlassian.net",
  "jira_auth": {
    "type": "bearer",
    "token": "abc123",
    "email": "test@test.com"
  },
  "column_mapping": {
    "summary_column": "A",
    "description_column": "B",
    "issue_type_column": "C"
  }
}
```

### Full Example (All Fields)
```json
{
  "workbook_id": "backlog.xlsx",
  "sheet_name": "Sprint 5",
  "jira_project_key": "PROJ",
  "jira_url": "https://company.atlassian.net",
  "jira_auth": {
    "type": "bearer",
    "token": "your-api-token",
    "email": "you@company.com"
  },
  "column_mapping": {
    "summary_column": "A",
    "description_column": "B",
    "issue_type_column": "C",
    "priority_column": "D",
    "assignee_column": "E",
    "labels_column": "F",
    "epic_link_column": "G",
    "story_points_column": "H"
  },
  "dry_run": false,
  "start_row": 2,
  "max_tickets": 50
}
```

### Dry Run Example
```json
{
  "workbook_id": "test.xlsx",
  "sheet_name": "Validation",
  "jira_project_key": "TEST",
  "jira_url": "https://test.atlassian.net",
  "jira_auth": {
    "type": "basic",
    "username": "admin",
    "password": "admin"
  },
  "column_mapping": {
    "summary_column": "A",
    "description_column": "B",
    "issue_type_column": "C"
  },
  "dry_run": true,
  "start_row": 2,
  "max_tickets": 5
}
```

## Troubleshooting

### No Tickets Created
- **Check start_row**: Default is 2 (skips header)
- **Verify data**: Ensure summary + issue_type are non-empty
- **Check max_tickets**: Increase if limiting batch size

### Authentication Errors
- **Atlassian Cloud**: Use API token + email
- **Personal Access Token**: Use token only (no email)
- **Jira Server/DC**: Use Basic Auth with username/password
- **Verify permissions**: Ensure token has "write:jira-work" scope

### Field Validation Errors
- **Issue Type**: Must match project configuration (Story/Bug/Task)
- **Priority**: Must match Jira priority scheme (Highest/High/Medium/Low/Lowest)
- **Assignee**: Must be valid Jira username (not display name)
- **Epic Link**: Must be existing epic key (e.g., "EPIC-123")

### Network Errors
- **Timeout**: Increase HTTP client timeout (modify `JIRA_API_TIMEOUT_SECS`)
- **DNS**: Verify Jira URL is reachable
- **Proxy**: Configure HTTP proxy if behind firewall

## Future Enhancements

### Planned Features
1. Custom field mapping
2. Bulk operations (update existing tickets)
3. Attachment support
4. Sprint assignment
5. Component/version mapping
6. OAuth 2.0 authentication
7. Jira Server/DC compatibility mode
8. Configurable rate limiting
9. Resumable batch processing (checkpoint/resume)
10. Rich text descriptions (markdown → ADF conversion)

### Contributions
See project CONTRIBUTING.md for contribution guidelines.

---

**Version**: 1.0.0
**Last Updated**: 2026-01-20
**Maintainer**: ggen-mcp project
