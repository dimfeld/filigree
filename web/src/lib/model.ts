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
  description: string;
  required: boolean;
}

export interface ModelDefinition<MODEL extends z.AnyZodObject> {
  name: string;
  plural: string;
  /** The base URL in the API for interacting with the model */
  url: string;
  fields: ModelField[];
  model: MODEL;
  /** Constraints to add to HTML fields for a particular field  */
  htmlConstraints?: {
    [name: string]: {
      min?: number;
      max?: number;
      required?: boolean;
    };
  };
}
