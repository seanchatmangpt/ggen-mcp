/**
 * Entity schemas for MCP domain
 * Auto-generated from ontology/mcp-domain.ttl
 */

export const EntitySchema = {
  Workbook: {
    type: 'object',
    properties: {
      id: { type: 'string', description: 'Unique workbook identifier' },
      path: { type: 'string', description: 'File system path' },
      sheets: {
        type: 'array',
        items: { $ref: '#/definitions/Sheet' },
        description: 'Collection of worksheets'
      },
      metadata: {
        type: 'object',
        properties: {
          created_at: { type: 'string', format: 'date-time' },
          modified_at: { type: 'string', format: 'date-time' }
        }
      }
    },
    required: ['id', 'path']
  },

  Sheet: {
    type: 'object',
    properties: {
      name: { type: 'string', description: 'Sheet name' },
      index: { type: 'integer', minimum: 0, description: 'Zero-based sheet index' },
      row_count: { type: 'integer', minimum: 0 },
      column_count: { type: 'integer', minimum: 0 }
    },
    required: ['name', 'index']
  },

  Cell: {
    type: 'object',
    properties: {
      address: { type: 'string', pattern: '^[A-Z]+[0-9]+$' },
      value: { type: ['string', 'number', 'boolean', 'null'] },
      formula: { type: 'string' },
      style: { $ref: '#/definitions/CellStyle' }
    },
    required: ['address']
  }
};
