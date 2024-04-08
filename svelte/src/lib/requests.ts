import {
  error,
  fail,
  type Cookies,
  type NumericRange,
  type RequestEvent,
  type ActionFailure,
} from '@sveltejs/kit';
import { client, type Client, type HttpMethod, type RequestOptions } from './client.js';
import parseCookie from 'set-cookie-parser';
import type { FormResponse } from './forms.svelte.js';

/** Forward a Request directly to an API endpoint.
 * @param method The HTTP method to use.
 * @param url The URL of the API endpoint. This function adds the `/api/` prefix.
 * @param event The event supplied by SvelteKit.
 * @param options Additional options for this request, which are passed to the client.
 * */
export function forwardToApi(
  method: HttpMethod,
  url: string,
  event: RequestEvent,
  options?: Partial<RequestOptions> & { client?: Client }
) {
  const thisClient = options?.client ?? client;

  let headers = new Headers(event.request.headers);

  // Let the server fetch implenentation manage these headers itself.
  // Most notably, modern browsers may accept certain encodings like zstd which
  // Node currently doesn't support, and not removing it breaks things when the
  // API sends back a response that Node can't decode.
  headers.delete('accept-encoding');
  headers.delete('connection');

  return thisClient({
    url: '/api/' + url,
    method,
    headers,
    signal: event.request.signal,
    body: event.request.body,
    fetch: event.fetch,
    ...options,
  });
}

export interface ErrorField<KIND extends string = string, DETAILS extends object = object> {
  kind: KIND;
  message: string;
  details: DETAILS;
}

export interface ErrorResponse<KIND extends string = string, DETAILS extends object = object> {
  error: ErrorField<KIND, DETAILS>;
}

/** Determine if a response from the API is an error of a particular kind */
export function isErrorResponse<T extends ErrorResponse<string, object>>(
  obj: object | null | undefined,
  kind?: string
): obj is T {
  if (!obj) {
    return false;
  }

  if (kind) {
    return obj && 'error' in obj && (obj.error as T['error'])?.kind === kind;
  } else {
    return obj && 'error' in obj;
  }
}

/** Data extracted from a response */
export interface ExtractedResponse<T> {
  type: 'extracted-response';
  status: number;
  body: T;
  headers: Headers;
}

export function isExtractedResponse<T>(
  t: ExtractedResponse<T> | ActionFailure<T>
): t is ExtractedResponse<T> {
  return t && 'type' in t && t.type === 'extracted-response';
}

/** Given the return value of handleFormResponse, return either the body or the ActionFailure */
export function getReturnBody<T>(
  value: ActionFailure<FormResponse<T>> | ExtractedResponse<FormResponse<T>>
) {
  if (isExtractedResponse(value)) {
    return value.body;
  } else {
    return value;
  }
}

/** Handle a form response, returning an ActionFailure if the status code matches validationFailureCodes,
 * throwing an error if the status code is some other failure code, or returning the body otherwise. */
export async function handleFormResponse<T extends object>(
  response: Response,
  validationFailureCodes: number[] = [400]
): Promise<ActionFailure<FormResponse<T>> | ExtractedResponse<FormResponse<T>>> {
  let { headers, status } = response;

  if (validationFailureCodes.includes(response.status)) {
    return fail<FormResponse<T>>(response.status, await response.json());
  } else if (!response.ok) {
    let err: object;
    try {
      err = await response.json();
    } catch (e) {
      err = { error: { kind: 'internal_error', message: response.statusText } };
    }

    error(response.status as NumericRange<400, 599>, err);
  }

  const body: FormResponse<T> = await response.json();

  return {
    type: 'extracted-response',
    body,
    headers,
    status,
  };
}

/** Forward a form request to an API endpoint, and check the response for errors and validation failures.
 * This is designed to be a "do-everything" function for standard form actions. More complex needs can be met by
 * calling `forwardToApi`, `handleFormResponse`, and `getReturnBody` as needed.
 *
 * @param method The HTTP method to use.
 * @param url The URL of the API endpoint. This function adds the `/api/` prefix.
 * @param event The event supplied by SvelteKit.
 * @param options Additional options for this request, which are passed to the client. Pass validationFailureCodes to override the default value of `[400]`.
 * */
export async function forwardFormToApi<T extends object>(
  method: HttpMethod,
  url: string,
  event: RequestEvent,
  options?: Partial<RequestOptions> & { client?: Client; validationFailureCodes?: number[] }
): Promise<ActionFailure<FormResponse<T>> | FormResponse<T>> {
  const response = await forwardToApi(method, url, event, {
    tolerateFailure: true,
    ...options,
  });

  const validationFailureCodes = options?.validationFailureCodes ?? [400];
  const r = await handleFormResponse<T>(response, validationFailureCodes);
  return getReturnBody(r);
}

/** Copy the Set-Cookie headers from the given response into the given headers. */
export function cookiesToHeaders(response: Response, headers?: HeadersInit): Headers {
  let h = headers instanceof Headers ? headers : new Headers(headers);
  let cookies = response.headers.getSetCookie();
  for (let cookie of cookies) {
    h.append('Set-Cookie', cookie);
  }
  return h;
}

/** Parse the cookies from a `Response` and add them to the given `Cookies` instance. */
export function applyResponseCookies(response: Response, cookies: Cookies) {
  let cookieHeader = response.headers.getSetCookie();
  let result = parseCookie(cookieHeader);

  for (let cookie of result) {
    if (!cookie.path) {
      cookie.path = '/';
    }

    if (cookie.value) {
      cookies.set(
        cookie.name,
        cookie.value,
        // @ts-expect-error
        cookie
      );
    } else {
      cookies.delete(
        cookie.name,
        // @ts-expect-error
        cookie
      );
    }
  }
}
