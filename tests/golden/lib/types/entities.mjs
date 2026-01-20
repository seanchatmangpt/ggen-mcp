/**
 * TypeScript type definitions for MCP entities
 * Auto-generated from ontology/mcp-domain.ttl
 */

export interface Workbook {
  id: string;
  path: string;
  sheets: Sheet[];
  metadata?: {
    created_at: string;
    modified_at: string;
  };
}

export interface Sheet {
  name: string;
  index: number;
  row_count: number;
  column_count: number;
}

export interface Cell {
  address: string;
  value?: string | number | boolean | null;
  formula?: string;
  style?: CellStyle;
}

export interface CellStyle {
  bold?: boolean;
  italic?: boolean;
  font_size?: number;
  background_color?: string;
}

export type CellValue = string | number | boolean | null;

export enum SheetType {
  Data = 'data',
  Calculator = 'calculator',
  Metadata = 'metadata'
}
