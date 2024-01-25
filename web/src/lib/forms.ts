import type { ModelDefinition } from './model.js';
import {
  type Client,
  client as defaultClient,
  type HttpMethod,
  HttpError,
  contextClient,
  client,
} from './client.js';
import { isErrorResponse, type ErrorResponse } from './requests.js';
import type { ErrorObject, ValidationError } from 'ajv';
import { applyAction, deserialize, enhance } from '$app/forms';
import type { ActionResult, SubmitFunction } from '@sveltejs/kit';
import { invalidateAll } from '$app/navigation';

export interface ValidationErrors {
  kind: 'validation';
  /** Validation messages not related to a specific field. */
  messages?: string[];
  /** Validation messages for particular fields. For nested data, the paths will be in JSON Pointer ('/' delimited) format. */
  fields?: Record<string, string[]>;
}

export type ValidationFailureResponse = ErrorResponse<'validation', ValidationErrors>;

export function isValidationFailure(obj: object | undefined): obj is ValidationFailureResponse {
  return isErrorResponse(obj, 'validation');
}

export interface FormResponse<MODEL extends object, ERROR = unknown | undefined> {
  form: MODEL;
  message?: string;
  toast?: never; // TODO make a toast type
  error: ERROR;
}

function isValidationError<T extends object>(
  obj: FormResponse<T> | undefined
): obj is FormResponse<T, ValidationErrors> {
  return !!obj?.error && 'kind' in obj && obj.kind === 'validation';
}

export type SubmitState = 'idle' | 'loading' | 'slow';

export interface FormOptions<T extends object> {
  model?: ModelDefinition<T>;
  /** If the form data is nested or flat. If omitted, it will be inferred from the model. */
  nested?: boolean;
  form?: FormResponse<T>;
  slowLoadThreshold?: number;

  /** Perform extra client-side validation. */
  validate?: (data: T) => ValidationErrors | undefined;

  resetForm?: boolean;
  invalidateAll?: boolean;

  onSubmit?: (args: Parameters<SubmitFunction>[0] & { data: T }) => void;
  onSuccess?: (result: ActionResult) => void | { resetForm?: boolean; invalidateAll?: boolean };
  onError?: (result: ActionResult) => void | ActionResult;
}

function processAjvError(errors: ErrorObject[]): ValidationErrors {
  let output: Record<string, string[]> = {};

  for (let error of errors) {
    let message = error.message ?? 'Invalid';

    let existing = output[error.instancePath];
    if (existing) {
      existing.push(message);
    } else {
      // TODO standardize messages with server version. Maybe just use WASM?
      output[error.instancePath] = [message];
    }
  }

  return {
    kind: 'validation',
    fields: output,
  };
}

interface State<T extends object> {
  message?: string;
  errors: Readonly<ValidationErrors | null>;
  form: T;
  loading: SubmitState;
}

interface InternalOptions<T extends object> {
  state: State<T>;
  options: FormOptions<T>;
  slowLoadThreshold: number;
}

export function manageForm<T extends object>(options: FormOptions<T>) {
  const formError = isValidationError(options.form) ? options.form.error : null;
  let state: State<T> = $state({
    message: options.form?.message,
    errors: formError,
    form: options.form?.form ?? ({} as T),
    loading: 'idle' as SubmitState,
  });

  // TODO If this was called with a toast, add the toast right away since it came from the server.

  let internalOptions: InternalOptions<T> = {
    state,
    options,
    slowLoadThreshold: options.slowLoadThreshold ?? 1000,
  };

  const nested = options.nested ?? options.model?.fields.some((f) => f.type === 'object') ?? false;

  let enhance: (formEl: HTMLFormElement) => void;
  // TODO Both of these enhance functions need to handle toast field in response once toast system is implemented
  if (nested) {
    enhance = nestedEnhance(internalOptions);
  } else {
    enhance = plainEnhance(internalOptions);
  }

  return {
    get errors() {
      return state.errors;
    },
    get message() {
      return state.message;
    },
    form: state.form,
    loading: $derived(state.loading === 'loading' || state.loading === 'slow'),
    slowLoading: $derived(state.loading === 'slow'),
    enhance,
  };
}

function validate<T extends object>(
  model: ModelDefinition<T> | undefined,
  options: InternalOptions<T>
) {
  if (!model) {
    return true;
  }

  let errors: ValidationErrors | undefined;

  let validated = model.validator(options.state.form);
  if (!validated) {
    errors = processAjvError(model.validator.errors ?? []);
  }

  let extraErrors = options.options.validate?.(options.state.form);
  if (extraErrors) {
    let errorFields = errors?.fields ?? {};
    for (let [key, value] of Object.entries(extraErrors.fields ?? {})) {
      if (errorFields[key]) {
        errorFields[key].push(...value);
      } else {
        errorFields[key] = value;
      }
    }

    let messages = [...(errors?.messages ?? []), ...(extraErrors.messages ?? [])];

    errors = {
      kind: 'validation',
      messages: messages.length ? messages : undefined,
      fields: errorFields,
    };
  }

  if (errors) {
    applyAction({
      type: 'failure',
      status: 400,
      data: {
        form: options.state.form,
        error: options.state.errors,
      },
    });

    return false;
  }

  return true;
}

function trackSlowLoading<T extends object>(options: InternalOptions<T>) {
  return setTimeout(() => {
    if (options.state.loading === 'loading') {
      options.state.loading = 'slow';
    }
  }, options.slowLoadThreshold);
}

function plainEnhance<T extends object>(options: InternalOptions<T>) {
  const {
    state,
    options: {
      model,
      onSubmit,
      onSuccess,
      onError,
      resetForm: resetFormOption,
      invalidateAll: invalidateAllOption,
    },
  } = options;

  return function (formEl: HTMLFormElement) {
    return enhance(formEl, (submitData) => {
      let cancelled = false;
      const hookCancel = () => {
        cancelled = true;
        submitData.cancel();
      };

      let validated = validate(model, options);
      if (!validated) {
        submitData.cancel();
        return;
      }

      onSubmit?.({ ...submitData, cancel: hookCancel, data: options.state.form });
      if (cancelled) {
        return;
      }

      state.loading = 'loading';
      let slowTimer = trackSlowLoading(options);

      return ({ result, update }) => {
        clearTimeout(slowTimer);
        state.loading = 'idle';

        let resetForm = resetFormOption ?? true;
        let invalidateAll = invalidateAllOption ?? true;

        if (result.type === 'failure' || result.type === 'error') {
          let hookResult = onError?.(result);
          if (hookResult) {
            result = hookResult;
          }

          if (result.type === 'failure') {
            const data = result.data as unknown as FormResponse<T>;
            state.message = data.message;
            if (isValidationError(data)) {
              state.errors = data.error;
            } else {
              state.errors = null;
            }
          }
        } else if (result.type === 'success') {
          const data = result.data as unknown as FormResponse<T>;
          state.message = data.message;

          let hookResult = onSuccess?.(result);
          if (hookResult) {
            if (typeof hookResult.invalidateAll === 'boolean') {
              invalidateAll = hookResult.invalidateAll;
            }

            if (typeof hookResult.resetForm === 'boolean') {
              resetForm = hookResult.resetForm;
            }
          }

          state.errors = null;
          update({
            reset: resetForm,
            invalidateAll,
          });
        }
      };
    });
  };
}

// For nested data, we need to send as JSON.
function nestedEnhance<T extends object>(options: InternalOptions<T>) {
  const {
    state,
    options: {
      model,
      onSubmit,
      onSuccess,
      onError,
      resetForm: resetFormOption,
      invalidateAll: invalidateAllOption,
    },
  } = options;

  return function (formEl: HTMLFormElement) {
    async function handleSubmit(event: SubmitEvent) {
      event.preventDefault();

      let cancelled = false;
      const cancel = () => (cancelled = true);

      let payload = state.form;

      let validated = validate(model, options);
      if (!validated) {
        return;
      }

      const submitter = event.submitter as HTMLButtonElement | undefined;
      const method =
        (submitter?.hasAttribute('formMethod') && submitter?.formMethod) || formEl.method;
      const action = new URL(
        (submitter?.hasAttribute('formaction') && submitter?.formAction) || formEl.action
      );
      const abort = new AbortController();

      onSubmit?.({
        action: new URL(action),
        controller: abort,
        formElement: formEl,
        submitter: event.submitter,
        cancel,
        data: options.state.form,
      });

      if (cancelled) {
        return;
      }

      state.loading = 'loading';
      const slowTimer = trackSlowLoading(options);

      let result: ActionResult;
      try {
        let response = await client({
          url: action,
          json: payload,
          method: method as HttpMethod,
          headers: {
            'x-sveltekit-action': 'true',
          },
          tolerateFailure: true,
          followRedirects: false,
          cache: 'no-store',
          abort,
        });

        result = deserialize(await response.text());

        clearTimeout(slowTimer);
        state.loading = 'idle';

        if (result.type === 'error') {
          result.status = response.status;
        }
      } catch (e) {
        clearTimeout(slowTimer);
        state.loading = 'idle';

        if ((e as Error).name === 'AbortError') {
          return;
        }

        result = { type: 'error', error: e };
      }

      if (result.type === 'success') {
        const resultData = result.data as unknown as FormResponse<T>;
        state.message = resultData.message;
        let shouldInvalidateAll = options.options.invalidateAll ?? true;
        let resetForm = options.options.resetForm ?? true;

        state.errors = null;
        state.form = resultData.form;

        let hookResult = onSuccess?.(result);
        if (hookResult) {
          if (typeof hookResult.invalidateAll === 'boolean') {
            shouldInvalidateAll = hookResult.invalidateAll;
          }

          if (typeof hookResult.resetForm === 'boolean') {
            resetForm = hookResult.resetForm;
          }
        }

        if (resetForm) {
          formEl.reset();
        }
        if (shouldInvalidateAll) {
          await invalidateAll();
        }
      } else if (result.type === 'failure' || result.type === 'error') {
        state.message = result.data?.message;

        let hookResult = onError?.(result);
        if (hookResult) {
          result = hookResult;
        }

        if (result.type === 'failure' && result.status === 400 && isValidationError(result.data)) {
          state.errors = result.data.error;
        }
      }

      await applyAction(result);
    }

    formEl.addEventListener('submit', handleSubmit);
    return {
      destroy() {
        formEl.removeEventListener('submit', handleSubmit);
      },
    };
  };
}
