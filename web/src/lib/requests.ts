// The interface here is somewhat inspired by Sindre Sorhus's excellent ky package. Any similarities are probably
// intentional. :) The primary differences are:
// - This package does not throw an error when fetching from a URL without a host and passing `query`. This comes up a
// lot in SvelteKit.
// - Option to declare which specific HTTP status codes should throw an error or not.

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
  prefixUrl: string;
  /** The default timeout to use for requests */
  timeout?: number;
  headers?: HeadersInit;
  retry?: number | RetryOptions;
  hooks?: {
    beforeRequest?: BeforeRequestHook[];
    beforeRetry?: BeforeRetryHook[];
    beforeError?: BeforeErrorHook[];
    afterResponse?: AfterResponseHook[];
  };
}

export type BeforeRequestHook = (
  request: Request,
  options: RequestOptions
) => Request | Response | undefined;

export interface BeforeRetryHookOptions {
  request: Request;
  options: RequestOptions;
  response?: Response;
  retryCount: number;
}
export type BeforeRetryHook = (options: BeforeRetryHookOptions) => void | Promise<void>;

export type BeforeErrorHook = (error: HttpError) => void | HttpError;

export type AfterResponseHook = (
  request: Request,
  options: RequestOptions,
  response: Response
) => void | Response | Promise<void | Response>;

export type SearchParamsInit =
  | string
  | Record<string, string | number | boolean | (string | number | boolean)[]>
  | [string, string][]
  | URLSearchParams;

export interface RetryOptions {
  limit?: number;
  methods?: HttpMethod[];
  statusCodes?: number[];
  maxRetryAfter?: number;
  backoffLimit?: number;
  delay?: (attemptCount: number) => number;
}

export interface RequestOptions {
  fetch?: typeof fetch;
  url: string | URL;
  method?: HttpMethod;
  headers?: HeadersInit;
  timeout?: number;
  json?: object;
  body?: BodyInit;
  abort?: AbortController;
  query?: SearchParamsInit;
  retry?: number | RetryOptions;
  /** If false or omitted, throw an error on any 4xx or 5xx status code (after retries, if applicable).
   * If true, failed responses are returned to the user.
   * If an array, failed responses with status codes in the array are returned to the user, and other failure codes
   * throw an error. */
  tolerateFailure?: boolean | number[];
}

function makeUrl(baseUrl: string, url: string | URL, searchParams: URLSearchParams) {
  searchParams.sort();
  let qs = searchParams.toString();

  if (url instanceof URL || url.startsWith('/') || url.includes('://')) {
    let result = new URL(url, baseUrl);
    result.search = qs;
    return result;
  } else {
    let search = qs ? `?${qs}` : '';
    return `${baseUrl}/${url}${search}`;
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
  request: Request;
  constructor(request: Request) {
    super('Timed out');
    this.request = request;
  }
}

export class RateLimitError extends Error {
  request: Request;
  response: Response;
  retryAfter: number;
  constructor(seconds: number, request: Request, response: Response) {
    super(`Rate limit exceeded, can retry in ${seconds} seconds`);
    this.request = request;
    this.response = response;
    this.retryAfter = seconds;
  }
}

export class HttpError extends Error {
  request: Request;
  response: Response;

  constructor(request: Request, response: Response) {
    super(`Request failed with status code ${response.status}`);
    this.request = request;
    this.response = response;
  }
}

async function wrapRetry(
  options: RequestOptions,
  retryOptions: RetryOptions | number | undefined,
  signal: AbortSignal,
  method: HttpMethod,
  timeout: number | undefined,
  hooks: BeforeRetryHook[] | undefined,
  thisFetch: typeof fetch,
  makeRequest: () => { request: Request; response: Response | null }
): Promise<{ request: Request; response: Response }> {
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

    const canRetry = canRetryMethod && currentTry <= limit;

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

    let fetchPromise = thisFetch(request);
    let response: Response | typeof TIMEOUT | undefined;
    lastResponse = undefined;
    try {
      response = await (timeout
        ? Promise.race([fetchPromise, sleep(timeout, signal)])
        : fetchPromise);
    } catch (e) {
      // fetch threw an error, which means we didn't even get to the point of receiving a response
      // Usually a network error, invalid host, etc.
      if (canRetry) {
        continue;
      } else {
        throw e;
      }
    }

    if (response === TIMEOUT) {
      // Don't retry on a time out, just throw the error
      throw new TimeoutError(request);
    }

    if (!canRetry || !statusCodes.includes(response.status)) {
      return { response, request };
    }

    lastResponse = response;

    // On a failure code, retry if we can.

    // Honor the Retry-After header if present.
    let retryAfterHeader = response.headers.get('Retry-After');
    let retryAfter = Number(retryAfterHeader);
    if (Number.isNaN(retryAfter) && retryAfterHeader) {
      retryAfter = (Date.parse(retryAfterHeader) - Date.now()) / 1000;
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

    await sleep(sleepTime * 1000, signal);

    currentTry += 1;
  }
}

export interface FetchPromise extends Promise<Response> {
  abort(): void;
  text(): Promise<string>;
  json<T>(): Promise<T>;
}

export interface Client {
  (options: RequestOptions): FetchPromise;
  extend(clientOptions: ClientOptions): Client;
}

function iterateHeadersInit(
  init: HeadersInit | undefined,
  cb: (key: string, value: string) => void
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

function updateHeaders(existing: Headers, newValue: HeadersInit | undefined) {
  iterateHeadersInit(newValue, (key, value) => {
    if (value === undefined) {
      existing.delete(key);
    } else {
      existing.set(key, value);
    }
  });
}

export function makeClient(clientOptions: ClientOptions) {
  const {
    prefixUrl: baseUrl,
    timeout: defaultTimeout,
    hooks,
    headers: headerOption,
  } = clientOptions;
  const fixedHeaders = new Headers(headerOption);
  fixedHeaders.set('Accept', 'application/json');

  const client = (options: RequestOptions) => {
    let abort = options.abort ?? new AbortController();

    let qs = makeSearchParams(options.query);
    let url = makeUrl(baseUrl, options.url, qs);

    let headers = new Headers(fixedHeaders);
    updateHeaders(headers, headerOption);

    let autoDetectContentType = !headers.has('Content-Type');
    let body: BodyInit | undefined;
    if (options.body) {
      body = options.body;

      if (autoDetectContentType) {
        if (body instanceof FormData) {
          headers.set('Content-Type', 'multipart/form-data');
        } else if (body instanceof URLSearchParams) {
          headers.set('Content-Type', 'application/x-www-form-urlencoded');
        }
      }
    } else if (options.json) {
      body = JSON.stringify(options.json);
      if (autoDetectContentType) {
        headers.set('Content-Type', 'application/json');
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
      let req = new Request(url, {
        method,
        signal: abort.signal,
        body,
      });

      for (let hook of beforeRequestHooks) {
        let hookResult = hook(req, options);
        if (hookResult instanceof Request) {
          req = hookResult;
          break;
        } else if (hookResult instanceof Response) {
          return { request: req, response: hookResult };
        }
      }

      return { request: req, response: null };
    }

    async function runRequest() {
      let { request, response: res } = await wrapRetry(
        options,
        options.retry ?? clientOptions.retry,
        abort.signal,
        method,
        timeout,
        clientOptions.hooks?.beforeRetry,
        thisFetch,
        makeRequest
      );

      for (let hook of clientOptions.hooks?.afterResponse ?? []) {
        let hookResult = await hook(request, options, res);
        if (hookResult) {
          res = hookResult;
        }
      }

      if (res.status < 400 || options.tolerateFailure === true) {
        return res;
      }

      if (Array.isArray(options.tolerateFailure) && options.tolerateFailure.includes(res.status)) {
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
    promise.abort = () => abort.abort();
    promise.text = () => promise.then((r) => r.text());
    promise.json = <T>() => promise.then((res) => res.json() as Promise<T>);
    return promise;
  };

  client.extend = (newOptions: ClientOptions) => {
    let headers = new Headers(fixedHeaders);
    updateHeaders(headers, newOptions.headers);

    return makeClient({ ...clientOptions, ...newOptions, headers });
  };

  client.create = makeClient;

  return client;
}
