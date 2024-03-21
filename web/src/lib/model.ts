import { z } from 'zod';

export type FieldType =
  | 'text'
  | 'integer'
  | 'float'
  | 'boolean'
  | 'date-time'
  | 'date'
  | 'uuid'
  | 'object'
  | 'blob';

export interface ModelField {
  name: string;
  type: FieldType;
  label: string;
  description?: string;
  /** Constraints to add to HTML fields for a particular field  */
  constraints?: {
    min?: number;
    max?: number;
    required?: boolean;
  };
}

export interface ModelDefinition<SCHEMA extends z.AnyZodObject> {
  name: string;
  plural: string;
  /** The base URL in the API for interacting with the model */
  url: string;
  fields: ModelField[];
  schema: SCHEMA;
}
