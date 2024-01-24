import type { ZodIssue } from 'zod';
import type { ModelDefinition } from './model.js';
import { type Client, client as defaultClient, type HttpMethod, HttpError } from './client.js';
import { isErrorResponse, type ErrorResponse } from './requests.js';

export interface ValidationErrors {
  /** Validation messages not related to a specific field. */
  messages?: string[];
  /** Validation messages for particular fields. For nested data, the paths will be in JSON Pointer ('/' delimited) format. */
  fields?: Record<string, string[]>;
}

export type ValidationFailureResponse = ErrorResponse<'validation', ValidationErrors>;

export function isValidationFailure(obj: object): obj is ValidationFailureResponse {
  return isErrorResponse(obj, 'validation');
}

export function apiEnhance(apiUrl: string) {
  // Similar to SvelteKit's enhance function but goes to the Rust server instead
}

export interface FormResponse<MODEL extends object> {
  data: MODEL;
  errors?: ValidationErrors;
}

export type SubmitState = 'idle' | 'loading' | 'slow';

export interface FormOptions<T extends object> {
  model: ModelDefinition<T>;
  formResponse?: FormResponse<T>;
  slowLoadThreshold?: number;
  client?: Client;
}

function processZodErrors(errors: ZodIssue[]): ValidationErrors {
  let output: Record<string, string[]> = {};

  for (let error of errors) {
    let path = error.path.join('/');
    let existing = output[path];
    if (existing) {
      existing.push(error.message);
    } else {
      // TODO standardize messages with server version. Maybe just use WASM?
      output[path] = [error.message];
    }
  }

  return {
    fields: output,
  };
}

export function form<T extends object>(options: FormOptions<T>) {
  let errors = $state.frozen(null as ValidationErrors | null);
  let form = $state(options.formResponse?.data ?? {});
  let loading = $state('idle' as SubmitState);

  const model = options.model;
  const client = options.client ?? defaultClient;

  return {
    errors,
    form,
    loading: $derived(loading === 'loading' || loading === 'slow'),
    slowLoading: $derived(loading === 'slow'),
    // TODO implement enhance function
    enhance: function (formEl: HTMLFormElement) {
      formEl.addEventListener('submit', async (event) => {
        event.preventDefault();

        let validated = await model.validator.safeParseAsync(form);
        if (!validated.success) {
          // TODO actually format the ZodError as errors
          errors = processZodErrors(validated.error.errors);
          return;
        }

        const formAction = (event.submitter as HTMLButtonElement)?.formAction || formEl.action;
        let payload = validated.data;

        loading = 'loading';
        let slowTimer = setTimeout(() => {
          if (loading === 'loading') {
            loading = 'slow';
          }
        });
        try {
          let result = await client({
            url: formAction,
            json: payload,
            method: formEl.method as HttpMethod,
            tolerateFailure: [400],
          }).json<T>();

          form = result;
          errors = null;

          loading = 'idle';
          clearTimeout(slowTimer);
        } catch (e) {
          loading = 'idle';
          clearTimeout(slowTimer);

          if (e instanceof HttpError && e.response.status === 400) {
            try {
              let response = await e.response.json();
              if (isValidationFailure(response)) {
                errors = response.error.details;
                return;
              }
            } catch (e) {
              // nothing to do here.
            }
          }

          // TODO Some other error occurred
          errors = {
            messages: [e.message],
          };
        }
      });

      return {};
    },
  };
}
