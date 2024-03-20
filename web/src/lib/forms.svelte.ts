import type { ModelDefinition } from './model.js';
import { z } from 'zod';
import { type HttpMethod, client } from './client.js';
import { isErrorResponse, type ErrorField, type ErrorResponse } from './requests.js';
import { applyAction, deserialize, enhance } from '$app/forms';
import type { ActionResult, SubmitFunction } from '@sveltejs/kit';
import { invalidateAll } from '$app/navigation';

export interface FormErrorDetails {
  /** Validation messages not related to a specific field. */
  messages?: string[];
  /** Validation messages for particular fields. For nested data, the paths will be in JSON Pointer ('/' delimited) format. */
  fields?: Record<string, string[]>;
}

export interface ErrorState<KIND extends string> {
  kind: KIND;
}

export interface FormErrors extends FormErrorDetails {
  kind: 'error' | 'validation';
}

export function isValidationFailure(
  obj: object | null | undefined
): obj is ErrorResponse<'validation', FormErrorDetails> {
  return isErrorResponse(obj, 'validation');
}

export interface FormResponse<MODEL extends object, ERROR = unknown> {
  form: Partial<MODEL>;
  message?: string;
  toast?: never; // TODO make a toast type
  error?: ERROR;
}

function isValidationError<T extends object>(
  obj: FormResponse<T> | null | undefined
): obj is FormResponse<T, ErrorField<'validation', FormErrorDetails>> {
  return isValidationFailure(obj);
}

export type SubmitState = 'idle' | 'loading' | 'slow';

export interface FormOptions<T extends z.AnyZodObject> {
  model?: ModelDefinition<T>;
  /** If the form data is nested or flat. If omitted, it will be inferred from the model. */
  nested?: boolean;
  /** Initial data to load into the form fields */
  data?: T | null;
  /** The page's form property */
  form?: FormResponse<T> | null;
  slowLoadThreshold?: number;

  /** Perform extra client-side validation. */
  validate?: (data: Partial<T>) => FormErrors | undefined;

  /** Whether or not to reset the form when the form is submitted.
   *
   * @default true */
  resetForm?: boolean;
  /** Whether or not to invalidate all loaded data when the form is submitted.
   *
   * @default true */
  invalidateAll?: boolean;

  onSubmit?: (args: Parameters<SubmitFunction>[0] & { data: T }) => void;
  onSuccess?: (result: ActionResult) => void | { resetForm?: boolean; invalidateAll?: boolean };
  onError?: (result: ActionResult) => void | ActionResult;
}

function processZodError(errors: z.ZodIssue[]): FormErrors {
  let output: Record<string, string[]> = {};

  for (let error of errors) {
    let message = error.message ?? 'Invalid';

    let path = error.path.join('/');
    let existing = output[path];
    if (existing) {
      existing.push(message);
    } else {
      // TODO standardize messages with server version. Maybe just use WASM?
      output[path] = [message];
    }
  }

  return {
    kind: 'validation',
    fields: output,
  };
}

class State<T extends z.AnyZodObject> {
  message: string | undefined = $state();
  errors: Readonly<FormErrors | null | undefined> = $state(null);
  fieldErrors = $derived(
    Object.fromEntries(Object.entries(this.errors?.fields ?? {}).map(([k, v]) => [k, v.join('\n')]))
  );
  formData: Partial<T> = $state({});
  loadingState: SubmitState = $state('idle');

  loading = $derived(this.loadingState === 'loading' || this.loadingState === 'slow');
  slowLoading = $derived(this.loadingState === 'slow');

  enhance: (formEl: HTMLFormElement) => void;

  constructor(options: FormOptions<T>) {
    // TODO If this was called with a toast, add the toast right away since it came from the server.
    this.message = options.form?.message;
    this.errors = errorStateFromErrorResponse(options.form);
    this.formData = options.form?.form ?? options.data ?? ({} as T);

    let internalOptions: InternalOptions<T> = {
      state: this,
      options,
      slowLoadThreshold: options.slowLoadThreshold ?? 1000,
    };

    const nested =
      options.nested ?? options.model?.fields.some((f) => f.type === 'object') ?? false;

    // TODO Both of these enhance functions need to handle toast field in response once toast system is implemented
    if (nested) {
      this.enhance = nestedEnhance(internalOptions);
    } else {
      this.enhance = plainEnhance(internalOptions);
    }
  }
}

interface InternalOptions<T extends z.AnyZodObject> {
  state: State<T>;
  options: FormOptions<T>;
  slowLoadThreshold: number;
}

export function manageForm<T extends z.AnyZodObject>(options: FormOptions<T>) {
  return new State(options);
}

function validate<T extends z.AnyZodObject>(
  model: ModelDefinition<T> | undefined,
  options: InternalOptions<T>
) {
  if (!model) {
    return true;
  }

  let errors: FormErrors | undefined;

  let validated = model.model.safeParse(options.state.formData);
  if (!validated.success) {
    errors = processZodError(validated.error.issues);
  }

  let extraErrors = options.options.validate?.(options.state.formData);
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
        form: options.state.formData,
        error: options.state.errors,
      },
    });

    return false;
  }

  return true;
}

function trackSlowLoading<T extends z.AnyZodObject>(options: InternalOptions<T>) {
  return setTimeout(() => {
    if (options.state.loadingState === 'loading') {
      options.state.loadingState = 'slow';
    }
  }, options.slowLoadThreshold);
}

function resolveSuccessHookReturnValue<T extends z.AnyZodObject>(
  options: InternalOptions<T>,
  retVal: { resetForm?: boolean; invalidateAll?: boolean } | void
) {
  return {
    resetForm: retVal?.resetForm ?? options.options.resetForm ?? true,
    invalidateAll: retVal?.invalidateAll ?? options.options.invalidateAll ?? true,
  };
}

function processErrorField(error: ErrorField<'validation', FormErrorDetails>) {
  let { details, ...rest } = error;

  let output: Record<string, string[]> = {};
  for (let key in error.details.fields) {
    const value = error.details.fields[key];
    // Simplify JSON pointer syntax a bit
    if (key[0] === '/') {
      key = key.slice(1);
    }

    output[key] = value;
  }

  return {
    ...rest,
    ...details,
    fields: output,
  };
}

function errorStateFromErrorResponse<T extends object>(
  data: FormResponse<T> | null | undefined
): FormErrors | null | undefined {
  if (data?.error == null) {
    return data?.error;
  }

  if (isValidationError(data)) {
    return processErrorField(data.error);
  } else {
    const message =
      typeof data.error === 'object' && data.error && 'message' in data.error
        ? (data.error.message as string)
        : 'An error occurred. Please try again';
    return {
      kind: 'error',
      messages: [message],
    };
  }
}

function maybeHandleFailureResult<T extends z.AnyZodObject>(
  options: InternalOptions<T>,
  result: ActionResult
) {
  if (result.type !== 'failure') {
    return;
  }

  const { state } = options;
  state.message = result.data?.message;

  const data = result.data as unknown as FormResponse<T>;
  state.errors = errorStateFromErrorResponse(data);
}

function plainEnhance<T extends z.AnyZodObject>(options: InternalOptions<T>) {
  const {
    state,
    options: { model, onSubmit, onSuccess, onError },
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

      onSubmit?.({ ...submitData, cancel: hookCancel, data: state.formData as T });
      if (cancelled) {
        return;
      }

      state.loadingState = 'loading';
      let slowTimer = trackSlowLoading(options);

      return async ({ result, update }) => {
        clearTimeout(slowTimer);
        state.loadingState = 'idle';

        if (result.type === 'error') {
          if (onError) {
            let hookResult = onError?.(result);
            if (hookResult) {
              result = hookResult;
            }
          } else {
            return applyAction(result);
          }
        } else if (result.type === 'failure') {
          if (onError) {
            let hookResult = onError?.(result);
            if (hookResult) {
              result = hookResult;
            }
          }

          maybeHandleFailureResult(options, result);
          update();
        } else if (result.type === 'success' || result.type === 'redirect') {
          if (result.type === 'success') {
            const data = result.data as unknown as FormResponse<T>;
            state.message = data.message;
            state.errors = null;
          }

          const hookResult = onSuccess?.(result);
          const updateOptions = resolveSuccessHookReturnValue(options, hookResult);

          update({
            reset: updateOptions.resetForm,
            invalidateAll: updateOptions.invalidateAll,
          });
        }
      };
    });
  };
}

// For nested data, we need to send as JSON.
function nestedEnhance<T extends z.AnyZodObject>(options: InternalOptions<T>) {
  const {
    state,
    options: { model, onSubmit, onSuccess, onError },
  } = options;

  return function (originalFormEl: HTMLFormElement) {
    async function handleSubmit(event: SubmitEvent) {
      event.preventDefault();

      let cancelled = false;
      const cancel = () => (cancelled = true);

      let payload = state.formData;

      let validated = validate(model, options);
      if (!validated) {
        return;
      }

      // Clone the node, so that any children whose names conflict with normal form properties
      // won't cause problems. (https://github.com/sveltejs/kit/pull/7599)
      let formEl = HTMLFormElement.prototype.cloneNode.call(originalFormEl) as HTMLFormElement;

      const submitter = event.submitter as HTMLButtonElement | undefined;
      const method =
        (submitter?.hasAttribute('formmethod') && submitter?.formMethod) || formEl.method;
      const action = new URL(
        (submitter?.hasAttribute('formaction') && submitter?.formAction) || formEl.action
      );
      const abort = new AbortController();

      onSubmit?.({
        action: new URL(action),
        controller: abort,
        formElement: formEl,
        formData: new FormData(originalFormEl),
        submitter: event.submitter,
        cancel,
        data: options.state.formData as T,
      });

      if (cancelled) {
        return;
      }

      state.loadingState = 'loading';
      const slowTimer = trackSlowLoading(options);

      let result: ActionResult;
      try {
        let response = await client({
          url: action,
          json: payload,
          method: method as HttpMethod,
          headers: {
            // Make sure the request goes to +page.server.ts, not +server.ts, if both exist.
            // Per https://kit.svelte.dev/docs/form-actions#progressive-enhancement-custom-event-listener
            'x-sveltekit-action': 'true',
          },
          tolerateFailure: true,
          followRedirects: false,
          cache: 'no-store',
          abort,
        });

        result = deserialize(await response.text());

        clearTimeout(slowTimer);
        state.loadingState = 'idle';

        if (result.type === 'error') {
          result.status = response.status;
        }
      } catch (e) {
        clearTimeout(slowTimer);
        state.loadingState = 'idle';

        if ((e as Error).name === 'AbortError') {
          return;
        }

        result = { type: 'error', error: e };
      }

      if (result.type === 'success') {
        const data = result.data as unknown as FormResponse<T>;
        state.formData = {
          ...state.formData,
          ...data.form,
        };

        state.message = data.message;
        state.errors = null;

        let hookResult = onSuccess?.(result);
        const updateOptions = resolveSuccessHookReturnValue(options, hookResult);

        if (updateOptions.resetForm) {
          HTMLFormElement.prototype.reset.call(originalFormEl);
        }
        if (updateOptions.invalidateAll) {
          await invalidateAll();
        }
      } else if (result.type === 'failure' || result.type === 'error') {
        let hookResult = onError?.(result);
        if (hookResult) {
          result = hookResult;
        }

        maybeHandleFailureResult(options, result);
      }

      await applyAction(result);
    }

    originalFormEl.addEventListener('submit', handleSubmit);
    return {
      destroy() {
        originalFormEl.removeEventListener('submit', handleSubmit);
      },
    };
  };
}
