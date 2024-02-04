// The interface here is somewhat inspired by Sindre Sorhus's excellent ky package. Any similarities are probably
// intentional. :) The primary differences are:
// - This package does not throw an error when fetching from a URL without a host and passing `query`. This comes up a
// lot in SvelteKit.
// - Option to declare which specific HTTP status codes should throw an error or not.

import { getContext, setContext } from 'svelte';

export type HttpMethod =
  | 'GET'
  | 'HEAD'
  | 'OPTIONS'
  | 'POST'
  | 'PUT'
  | 'DELETE'
  | 'TRACE'
  | 'CONNECT';

export interface ClientOptions {
  /** A URL to prepend to all requests with relative URLs. */
  prefixUrl?: string;
  /** The default timeout to use for requests, in seconds */
  timeout?: number;
  /** Headers to pass with every request. */
  headers?: HeadersInit;
  /** Customize the retry behavior. Pass a number to define the maxium number of retries, or a {@link RetryOptions}
   * to fully customize the retry settings. */
  retry?: number | RetryOptions;
  /** If false or omitted, throw an error on any 4xx or 5xx status code (after retries, if applicable).
   * If true, failed responses are returned to the user.
   * If an array, failed responses with status codes in the array are returned to the user, and other failure codes
   * throw an error. */
  tolerateFailure?: boolean | number[];
  /** Hooks to customize request and response handling. */
  hooks?: {
    /** Called before a request is made. Here you can customize the existing Request or return a whole new Request or
     * Response. When returning a Response, the fetch API will not be called, and instead the Response will be returned
     * directly to the caller. */
    beforeRequest?: BeforeRequestHook[];
    /** Called before a retry is done. Here you can customize the Request before it is sent. This function may return a
     * Promise, to allow behavior such as fetching a new access token before retrying. */
    beforeRetry?: BeforeRetryHook[];
    /** Called when an HttpError occurs. */
    beforeError?: BeforeErrorHook[];
    /** Called with the Response returned from the fetch call. This can be used to inspect or modify the Response, or replace it
     * completely. */
    afterResponse?: AfterResponseHook[];
  };
}

export type BeforeRequestHook = (
  request: RequestInput,
  options: RequestOptions
) => Request | RequestInput | Response | undefined;

export interface BeforeRetryHookOptions {
  request: RequestInput;
  options: RequestOptions;
  response?: Response;
  retryCount: number;
}
export type BeforeRetryHook = (options: BeforeRetryHookOptions) => void | Promise<void>;

export type BeforeErrorHook = (error: HttpError) => void | HttpError;

export type AfterResponseHook = (
  request: RequestInput,
  options: RequestOptions,
  response: Response
) => void | Response | Promise<void | Response>;

export type SearchParamsInit =
  | string
  | Record<string, string | number | boolean | (string | number | boolean)[]>
  | [string, string][]
  | URLSearchParams;

/** A type that is similar to a Request, but can be constructed in Node.js without having an absolute URL. This is
 * useful for SvelteKit where it's common to pass a relative URL to its overridden fetch function. */
export type RequestInput = Request | (RequestInit & { url: string | URL });

export interface RetryOptions {
  limit?: number;
  methods?: HttpMethod[];
  statusCodes?: number[];
  maxRetryAfter?: number;
  backoffLimit?: number;
  delay?: (attemptCount: number) => number;
}

export interface RequestOptions {
  /** An alternate `fetch` function to use. This should be passed in when calling the client from SvelteKit load
   * functions, for example. */
  fetch?: typeof fetch;
  /** The URL to send the request to. If a relative URL, it will be appended to the prefix URL used when creating the
   * client.. */
  url: string | URL;
  /** The HTTP method to pass with this request. If omitted, the method will be GET. */
  method?: HttpMethod;
  /** Headers to pass with this request. */
  headers?: HeadersInit;
  /** The timeout, in seconds, for this request. */
  timeout?: number;
  /** An object to serialize and place into the body. */
  json?: object;
  /** The body to send with the request. If sending json, use the `json` option instead. */
  body?: BodyInit | null;
  /** Customize caching behavior. */
  cache?: RequestCache;
  /** An abort controller to use for the request. If omitted, a new one will be created, so {@link FetchPromise.abort()} will still
   * work. */
  abort?: AbortController;
  /** Supply a signal from an AbortController to cancel the request. This should only be
   * used if the AbortController that created the signal is not available to the caller.
   *
   * When `signal` is passed, the `abort` option is ignored and the `abort` method on the client's return value will do
   * nothing.
   * */
  signal?: AbortSignal;
  /** The query string to append to the URL. If this is present, the passed-in URL should not already have a query
   * string. */
  query?: SearchParamsInit;
  /** Customize the retry behavior. Pass a number to define the maxium number of retries, or a {@link RetryOptions}
   * to fully customize the retry settings. */
  retry?: number | RetryOptions;
  /** If false or omitted, throw an error on any 4xx or 5xx status code (after retries, if applicable).
   * If true, failed responses are returned to the user.
   * If an array, failed responses with status codes in the array are returned to the user, and other failure codes
   * throw an error. */
  tolerateFailure?: boolean | number[];
  followRedirects?: boolean;
}

function makeUrl(baseUrl: string | undefined, url: string | URL, searchParams: URLSearchParams) {
  searchParams.sort();
  let qs = searchParams.toString();

  if (url instanceof URL || url.includes('://')) {
    let result = new URL(url);
    result.search = qs;
    return result;
  } else {
    let search = qs ? `?${qs}` : '';
    let hasSlash = url.startsWith('/');
    if (hasSlash || !baseUrl) {
      return `${url}${search}`;
    } else {
      return `${baseUrl}/${url}${search}`;
    }
  }
}

function makeSearchParams(query: SearchParamsInit | undefined) {
  if (!query) return new URLSearchParams();
  if (query instanceof URLSearchParams) return query;
  if (typeof query === 'string' || Array.isArray(query)) {
    return new URLSearchParams(query);
  }

  let qs = new URLSearchParams();

  for (let [key, value] of Object.entries(query)) {
    if (Array.isArray(value)) {
      for (let v of value) {
        qs.append(key, v.toString());
      }
    } else {
      qs.set(key, value.toString());
    }
  }

  return qs;
}

const DEFAULT_RETRY_STATUS_CODES = [408, 413, 429, 500, 502, 503, 504];
const DEFAULT_RETRY_LIMIT = 2;
const DEFAULT_RETRY_METHODS = ['GET', 'PUT', 'HEAD', 'DELETE', 'OPTIONS', 'TRACE'];
const DEFAULT_DELAY = (attemptCount: number) => 0.3 * 2 ** (attemptCount - 1) * 1000;
const TIMEOUT = Symbol();

function sleep(time: number, signal: AbortSignal): Promise<typeof TIMEOUT> {
  return new Promise((resolve, reject) => {
    function aborted() {
      clearTimeout(timeout);
      reject(signal.reason);
    }

    signal?.throwIfAborted();
    signal?.addEventListener('abort', aborted, { once: true });

    let timeout = setTimeout(() => {
      signal?.removeEventListener('abort', aborted);
      resolve(TIMEOUT);
    }, time);
  });
}

export class TimeoutError extends Error {
  request: RequestInput;
  constructor(request: RequestInput) {
    super('Timed out');
    this.request = request;
  }
}

export class RateLimitError extends Error {
  request: RequestInput;
  response: Response;
  retryAfter: number;
  constructor(seconds: number, request: RequestInput, response: Response) {
    super(`Rate limit exceeded, can retry in ${seconds} seconds`);
    this.request = request;
    this.response = response;
    this.retryAfter = seconds;
  }
}

export class HttpError extends Error {
  request: RequestInput;
  response: Response;

  constructor(request: RequestInput, response: Response) {
    super(`Request failed with status code ${response.status}`);
    this.request = request;
    this.response = response;
  }
}

async function wrapRetry(
  options: RequestOptions,
  retryOptions: RetryOptions | number | undefined,
  tolerateFailure: boolean | number[],
  signal: AbortSignal,
  method: HttpMethod,
  timeout: number | undefined,
  hooks: BeforeRetryHook[] | undefined,
  thisFetch: typeof fetch,
  makeRequest: () => { request: RequestInput; response: Response | null }
): Promise<{ request: RequestInput; response: Response }> {
  const limit =
    typeof retryOptions === 'number' ? retryOptions : retryOptions?.limit ?? DEFAULT_RETRY_LIMIT;
  retryOptions = typeof retryOptions === 'object' ? retryOptions : {};

  const methods = retryOptions?.methods ?? DEFAULT_RETRY_METHODS;
  const statusCodes = retryOptions?.statusCodes ?? DEFAULT_RETRY_STATUS_CODES;
  const maxRetryAfter = retryOptions?.maxRetryAfter ?? timeout;
  const delay = retryOptions?.delay ?? DEFAULT_DELAY;
  const backoffLimit = retryOptions?.backoffLimit ?? Infinity;

  const canRetryMethod = methods.includes(method.toUpperCase());

  let lastResponse: Response | undefined;

  let currentTry = 0;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    let mr = makeRequest();
    if (mr.response) {
      // A hook returned a Response, so just return it.
      return { request: mr.request, response: mr.response };
    }

    const request = mr.request;

    const canRetry = canRetryMethod && currentTry < limit;

    if (currentTry > 0) {
      for (let retryHook of hooks ?? []) {
        await retryHook({
          request,
          options,
          response: lastResponse,
          retryCount: currentTry,
        });
      }
    }

    let fetchPromise =
      request instanceof Request ? thisFetch(request) : thisFetch(request.url, request);
    let response: Response | typeof TIMEOUT | undefined;
    lastResponse = undefined;
    try {
      response = await (timeout
        ? Promise.race([fetchPromise, sleep(timeout, signal)])
        : fetchPromise);
    } catch (e) {
      // fetch threw an error, which means we didn't even get to the point of receiving a response
      // Usually a network error, invalid host, etc.
      if (canRetry && !signal.aborted) {
        continue;
      } else {
        throw e;
      }
    }

    if (response === TIMEOUT) {
      // Don't retry on a time out, just throw the error
      throw new TimeoutError(request);
    }

    if (
      !canRetry ||
      !statusCodes.includes(response.status) ||
      tolerateFailure === true ||
      (Array.isArray(tolerateFailure) && tolerateFailure.includes(response.status))
    ) {
      return { response, request };
    }

    lastResponse = response;

    // On a failure code, retry if we can.

    // Honor the Retry-After header if present.
    let retryAfterHeader = response.headers.get('Retry-After');
    let retryAfter = Number(retryAfterHeader) * 1000;
    if (Number.isNaN(retryAfter) && retryAfterHeader) {
      retryAfter = Date.parse(retryAfterHeader) - Date.now();
    }

    let sleepTime: number | undefined;
    if (retryAfter) {
      if (maxRetryAfter && retryAfter > maxRetryAfter) {
        // We can't retry if the server says to wait longer than the limit.
        throw new RateLimitError(retryAfter, request, response);
      } else {
        sleepTime = retryAfter;
      }
    } else {
      if (response.status === 413) {
        // Payload was too large, and we didn't get a Retry-After header, so there's no point in trying again
        // since it will still be too large on the next try.
        return { request, response };
      }

      // Lacking a Retry-After header, sleep for the backoff time.
      sleepTime = sleepTime ?? Math.min(delay(currentTry + 1), backoffLimit);
    }

    await sleep(sleepTime, signal);

    currentTry += 1;
  }
}

export interface FetchPromise extends Promise<Response> {
  abort(): void;
  text(): Promise<string>;
  json<T>(): Promise<T>;
  arrayBuffer(): Promise<ArrayBuffer>;
  blob(): Promise<Blob>;
  formData(): Promise<FormData>;
}

export interface Client {
  (options: RequestOptions): FetchPromise;
  extend(clientOptions: ClientOptions): Client;
}

function iterateHeadersInit(
  init: Headers | [string, string | undefined][] | Record<string, string | undefined> | undefined,
  cb: (key: string, value: string | undefined) => void
) {
  if (init instanceof Headers) {
    for (let [key, value] of init.entries()) {
      cb(key, value);
    }
  } else if (Array.isArray(init)) {
    for (let [key, value] of init) {
      cb(key, value);
    }
  } else if (init) {
    for (let [key, value] of Object.entries(init)) {
      cb(key, value);
    }
  }
}

export function updateHeaders(existing: Headers, newValue: HeadersInit | undefined) {
  iterateHeadersInit(newValue, (key, value) => {
    if (value === undefined) {
      existing.delete(key);
    } else {
      existing.set(key, value);
    }
  });
}

export function mergeRetryOptions(
  base: number | RetryOptions | undefined,
  other: number | RetryOptions | undefined
): RetryOptions | number | undefined {
  if (other === undefined) {
    return base;
  } else if (base === undefined) {
    return other;
  } else if (typeof other === 'number') {
    if (typeof base === 'object') {
      return {
        ...base,
        limit: other,
      };
    } else {
      return other;
    }
  } else if (typeof base === 'number') {
    if (typeof other === 'object') {
      return {
        limit: base,
        ...other,
      };
    } else {
      return base;
    }
  } else {
    return {
      ...base,
      ...other,
    };
  }
}

/** Create an HTTP client with the given options. */
export function makeClient(clientOptions: ClientOptions = {}): Client {
  let { prefixUrl: baseUrl, timeout: defaultTimeout, hooks, headers: headerOption } = clientOptions;
  const fixedHeaders = new Headers(headerOption);
  if (!fixedHeaders.has('Accept')) {
    fixedHeaders.set('Accept', 'application/json');
  }

  if (baseUrl?.endsWith('/')) {
    baseUrl = baseUrl.slice(0, -1);
  }

  const client = (options: RequestOptions) => {
    const tolerateFailure = options.tolerateFailure ?? clientOptions.tolerateFailure ?? false;

    let abort: AbortController | undefined;
    let signal: AbortSignal;
    if (options.signal && options.signal !== options.abort?.signal) {
      signal = options.signal;
    } else {
      abort = options.abort ?? new AbortController();
      signal = abort.signal;
    }

    let qs = makeSearchParams(options.query);
    let url = makeUrl(baseUrl, options.url, qs);

    let headers = new Headers(fixedHeaders);
    updateHeaders(headers, options.headers);

    let body: BodyInit | undefined;
    if (options.body) {
      body = options.body;
    } else if (options.json) {
      body = JSON.stringify(options.json);
      if (!headers.has('Content-Type')) {
        headers.set('Content-Type', 'application/json;charset=UTF-8');
      }
    }

    let timeout = options.timeout ?? defaultTimeout;
    if (timeout) {
      timeout *= 1000;
    }

    const thisFetch = options.fetch ?? globalThis.fetch;
    const method = options.method ?? 'GET';

    const beforeRequestHooks = hooks?.beforeRequest ?? [];
    function makeRequest() {
      let req: RequestInput = {
        url,
        method,
        headers,
        body,
        cache: options.cache,
        signal,
        redirect: options.followRedirects === false ? 'manual' : 'follow',
      };

      for (let hook of beforeRequestHooks) {
        let hookResult = hook(req, options);
        if (hookResult instanceof Response) {
          return { request: req, response: hookResult };
        } else if (
          hookResult instanceof Request ||
          (hookResult && typeof hookResult === 'object')
        ) {
          req = hookResult;
          break;
        }
      }

      return { request: req, response: null };
    }

    async function runRequest(): Promise<Response> {
      let { request, response: res } = await wrapRetry(
        options,
        options.retry ?? clientOptions.retry,
        tolerateFailure,
        signal,
        method,
        timeout,
        clientOptions.hooks?.beforeRetry,
        thisFetch,
        makeRequest
      );

      for (let hook of clientOptions.hooks?.afterResponse ?? []) {
        let hookResult = await hook(request, options, res.clone());
        if (hookResult) {
          res = hookResult;
        }
      }

      if (res.status < 400 || tolerateFailure === true) {
        return res;
      }

      if (Array.isArray(tolerateFailure) && tolerateFailure.includes(res.status)) {
        return res;
      }

      let error = new HttpError(request, res);
      for (let hook of hooks?.beforeError ?? []) {
        let hookResult = hook(error);
        if (hookResult) {
          error = hookResult;
        }
      }

      throw error;
    }

    const promise = runRequest() as FetchPromise;
    promise.abort = () => abort?.abort();
    promise.arrayBuffer = () => promise.then((r) => r.arrayBuffer());
    promise.blob = () => promise.then((r) => r.blob());
    promise.formData = () => promise.then((res) => res.formData());
    promise.json = <T>() => promise.then((res) => res.json() as Promise<T>);
    promise.text = () => promise.then((r) => r.text());
    return promise;
  };

  client.extend = (newOptions: ClientOptions) => {
    let headers = new Headers(fixedHeaders);
    updateHeaders(headers, newOptions.headers);

    let retry = mergeRetryOptions(clientOptions.retry, newOptions.retry);

    return makeClient({ ...clientOptions, ...newOptions, headers, retry });
  };

  client.create = makeClient;

  return client;
}

/** A client with default options. */
export const client = makeClient();

const FILIGREE_CLIENT = Symbol('filigree-client');
/** Set a client in the Svelte context for child components to use. */
export function setContextClient(newClient: Client): Client {
  return setContext(FILIGREE_CLIENT, newClient);
}

/** Retrieve a client from the Svelte context. */
export function contextClient(): Client {
  return getContext(FILIGREE_CLIENT);
}
