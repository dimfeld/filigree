import AjvModule from 'ajv';

const ajv = new AjvModule.default({ allErrors: true });

export type FieldType = 'text' | 'integer' | 'float' | 'boolean' | 'date-time' | 'date' | 'object';

export interface ModelField {
  name: string;
  type: FieldType;
  description: string;
  required: boolean;
}

export interface ModelDefinition<MODEL> {
  name: string;
  plural: string;
  /** The base URL in the API for interacting with the model */
  url: string;
  fields: ModelField[];
  /** Constraints to add to HTML fields for a particular  */
  htmlConstraints?: {
    [name: string]: {
      min?: number;
      max?: number;
      required?: boolean;
    };
  };
  validator: AjvModule.ValidateFunction<MODEL>;
}
