import { ZodType } from 'zod';

export type FieldType =
  | 'string'
  | 'integer'
  | 'float'
  | 'boolean'
  | 'date-time'
  | 'date'
  | 'object';

export interface ModelField {
  name: string;
  type: FieldType;
  description: string;
  required: boolean;
}

export interface ModelDefinition<SCHEMA> {
  name: string;
  plural: string;
  /** The base URL in the API for interacting with the model */
  url: string;
  fields: ModelField[];
  htmlConstraints?: {
    [key: string]: {
      min?: number;
      max?: number;
      required?: boolean;
    };
  };
  validator: ZodType<SCHEMA>;
}
