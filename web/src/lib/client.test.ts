import { expect, test, describe, beforeEach, afterEach, vi } from 'vitest';
import {
  mergeRetryOptions,
  updateHeaders,
  type ClientOptions,
  makeClient,
  HttpError,
  TimeoutError,
  RateLimitError,
  type RequestInput,
} from './requests.js';
import sorter from 'sorters';

describe('mergeRetryOptions', () => {
  test('return undefined when both are undefined', () => {
    expect(mergeRetryOptions(undefined, undefined)).eq(undefined);
  });

  test('should return base if other is undefined', () => {
    expect(mergeRetryOptions(5, undefined)).eq(5);
  });

  test('should return other if base is undefined', () => {
    expect(mergeRetryOptions(undefined, 5)).eq(5);
  });

  test('base is object and other is number', () => {
    expect(mergeRetryOptions({ limit: 5, maxRetryAfter: 10 }, 7)).deep.eq({
      limit: 7,
      maxRetryAfter: 10,
    });
  });

  test('base is number and other is object without limit', () => {
    expect(mergeRetryOptions(7, { maxRetryAfter: 10 })).deep.eq({
      limit: 7,
      maxRetryAfter: 10,
    });
  });

  test('base is number and other is object with limit', () => {
    expect(mergeRetryOptions(7, { limit: 5, maxRetryAfter: 10 })).deep.eq({
      limit: 5,
      maxRetryAfter: 10,
    });
  });

  test('base is object and other is object', () => {
    expect(
      mergeRetryOptions({ limit: 5, maxRetryAfter: 10 }, { limit: 7, maxRetryAfter: 15 })
    ).deep.eq({
      limit: 7,
      maxRetryAfter: 15,
    });
  });
});

describe('updateHeaders', () => {
  function sortHeaders(h: Headers) {
    let entries = Array.from(h.entries());
    entries.sort(sorter((v) => v[0]));
    return entries;
  }

  test('update from other Headers', () => {
    let existing = new Headers({ accept: 'application/json', 'content-type': 'application/json' });
    let other = new Headers();
    other.append('accept', 'application/json');
    other.append('accept', 'application/xml');
    updateHeaders(existing, other);
    expect(sortHeaders(existing)).toEqual([
      ['accept', 'application/json, application/xml'],
      ['content-type', 'application/json'],
    ]);
  });

  test('update from array', () => {
    let existing = new Headers({ accept: 'application/json', 'content-type': 'application/json' });
    let other = [
      ['accept', 'text/plain'],
      ['content-type', undefined],
    ];
    updateHeaders(existing, other);
    expect(sortHeaders(existing)).toEqual([['accept', 'text/plain']]);
  });

  test('update from object', () => {
    let existing = new Headers({ accept: 'application/json', 'content-type': 'application/json' });
    let other = {
      accept: 'text/plain',
      'content-type': undefined,
    };
    updateHeaders(existing, other);
    expect(sortHeaders(existing)).toEqual([['accept', 'text/plain']]);
  });

  test('with undefined', () => {
    let existing = new Headers({ accept: 'application/json', 'content-type': 'application/json' });
    updateHeaders(existing, undefined);
    expect(sortHeaders(existing)).toEqual([
      ['accept', 'application/json'],
      ['content-type', 'application/json'],
    ]);
  });
});

describe('client', () => {
  function createClient(
    clientOptions: ClientOptions,
    response?: (url: string, req: RequestInput) => Response
  ) {
    let fetch = vi.fn(response ?? (() => new Response('{}', { status: 200 })));
    let client = makeClient(clientOptions);
    return { fetch, client };
  }

  test('base client', async () => {
    const { fetch, client } = createClient({});

    let response = await client({ url: '/abc', fetch });

    expect(response.status).eq(200);
    let args = fetch.mock.lastCall;
    expect(args[0]).eq('/abc');
    expect(args[1].method).eq('GET');
  });

  describe('prefixUrl', () => {
    test('with relative URL', async () => {
      const { fetch, client } = createClient({ prefixUrl: '/api' });

      let response = await client({ url: 'abc', fetch, query: { foo: 'bar' } });

      expect(response.status).eq(200);
      let args = fetch.mock.lastCall;
      expect(args[0]).eq('/api/abc?foo=bar');
      expect(args[1].method).eq('GET');
    });

    test('with absolute URL, no host', async () => {
      const { fetch, client } = createClient({ prefixUrl: '/api' });

      let response = await client({ url: '/abc', fetch, query: { foo: 'bar' } });

      expect(response.status).eq(200);
      let args = fetch.mock.lastCall;
      expect(args[0]).eq('/abc?foo=bar');
      expect(args[1].method).eq('GET');
    });

    test('with absolute URL, with host', async () => {
      const { fetch, client } = createClient({ prefixUrl: '/api' });

      let response = await client({ url: 'https://example.com/abc', fetch, query: { foo: 'bar' } });

      expect(response.status).eq(200);
      let args = fetch.mock.lastCall;
      expect(args[0].href).eq('https://example.com/abc?foo=bar');
      expect(args[1].method).eq('GET');
    });

    test('called with URL instance', async () => {
      const { fetch, client } = createClient({ prefixUrl: '/api' });

      let response = await client({
        url: new URL('https://example.com/abc'),
        fetch,
        query: { foo: 'bar' },
      });

      expect(response.status).eq(200);
      let args = fetch.mock.lastCall;
      expect(args[0].href).eq('https://example.com/abc?foo=bar');
      expect(args[1].method).eq('GET');
    });
  });

  test('timeout', async () => {
    vi.useFakeTimers();

    try {
      let retryHook = vi.fn();
      let { client, fetch } = createClient(
        { timeout: 20, hooks: { beforeRetry: [retryHook] } },
        // Promise that never resolves
        () => new Promise(() => {})
      );

      let currrentTime = Date.now();
      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      vi.runAllTimers();
      let result = await promise;
      expect(result).instanceOf(TimeoutError);
      let passed = Date.now() - currrentTime;
      expect(passed).closeTo(20000, 500, 'timeout check');
      expect(retryHook).not.toHaveBeenCalled();
    } finally {
      vi.restoreAllMocks();
    }
  });

  describe('retry', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });
    test('retry works on next try', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient({ hooks: { beforeRetry: [beforeRetry] } });

      fetch.mockImplementationOnce(() => new Response('{}', { status: 500 }));

      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let response = await promise;
      expect(response.status).eq(200);

      expect(fetch.mock.calls).length(2);
      expect(beforeRetry.mock.calls).length(1);
    });

    test('when retry never succeeds', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient(
        { hooks: { beforeRetry: [beforeRetry] } },
        () => new Response('{}', { status: 500 })
      );
      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let result: HttpError = await promise;
      expect(result).instanceOf(HttpError);
      expect(result.message).eq('Request failed with status code 500');

      expect(fetch.mock.calls).length(3);
      expect(beforeRetry.mock.calls).length(2);
    });

    test('set number of retries', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient(
        { retry: 5, hooks: { beforeRetry: [beforeRetry] } },
        () => new Response('{}', { status: 500 })
      );
      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let result: HttpError = await promise;
      expect(result).instanceOf(HttpError);
      expect(result.message).eq('Request failed with status code 500');

      expect(fetch.mock.calls).length(6);
      expect(beforeRetry.mock.calls).length(5);
    });

    test('set 0 retries', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient(
        { retry: 0, hooks: { beforeRetry: [beforeRetry] } },
        () => new Response('{}', { status: 500 })
      );
      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let result: HttpError = await promise;
      expect(result).instanceOf(HttpError);
      expect(result.message).eq('Request failed with status code 500');

      expect(fetch.mock.calls).length(1);
      expect(beforeRetry.mock.calls).length(0);
    });

    test('override status codes', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient(
        { retry: { statusCodes: [503] }, hooks: { beforeRetry: [beforeRetry] } },
        () => new Response('{}', { status: 500 })
      );

      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let result: HttpError = await promise;
      expect(result).instanceOf(HttpError);
      expect(result.message).eq('Request failed with status code 500');

      expect(fetch.mock.calls).length(1);
      expect(beforeRetry.mock.calls).length(0);
    });

    test('does not retry on POST', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient(
        { hooks: { beforeRetry: [beforeRetry] } },
        () => new Response('{}', { status: 500 })
      );

      let promise = client({ url: '/abc', method: 'POST', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let result: HttpError = await promise;
      expect(result).instanceOf(HttpError);
      expect(result.message).eq('Request failed with status code 500');

      expect(fetch.mock.calls).length(1);
      expect(beforeRetry.mock.calls).length(0);
    });

    test('override retry methods', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient(
        { retry: { methods: ['POST'] }, hooks: { beforeRetry: [beforeRetry] } },
        () => new Response('{}', { status: 500 })
      );

      let promise = client({ url: '/abc', method: 'POST', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let result: HttpError = await promise;
      expect(result).instanceOf(HttpError);
      expect(result.message).eq('Request failed with status code 500');

      expect(fetch.mock.calls).length(3);
      expect(beforeRetry.mock.calls).length(2);
    });

    test('retry-after', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient({
        hooks: { beforeRetry: [beforeRetry] },
      });

      fetch.mockImplementationOnce(
        () => new Response('wait', { status: 429, headers: { 'retry-after': '3' } })
      );

      let start = Date.now();
      let promise = client({ url: '/abc', fetch });
      await vi.advanceTimersByTimeAsync(3000);
      let result = await promise;
      let passed = Date.now() - start;
      expect(result.status).eq(200);

      expect(fetch.mock.calls).length(2);
      expect(beforeRetry.mock.calls).length(1);
      expect(passed).closeTo(3000, 50);
    });

    test('retry-after too long', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient({
        retry: { maxRetryAfter: 10 },
        hooks: { beforeRetry: [beforeRetry] },
      });

      fetch.mockImplementationOnce(
        () => new Response('wait', { status: 429, headers: { 'retry-after': '300' } })
      );

      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(3000);
      let result = await promise;
      expect(result).instanceOf(RateLimitError);

      expect(fetch.mock.calls).length(1);
      expect(beforeRetry.mock.calls).length(0);
    });

    test('aborted while waiting for retry', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient({ hooks: { beforeRetry: [beforeRetry] } });

      fetch.mockImplementationOnce(() => new Response('{}', { status: 500 }));

      let promise = client({ url: '/abc', fetch });
      await vi.advanceTimersByTimeAsync(100);
      promise.abort();

      let response = await promise.catch((e) => e);
      expect(response.name).eq('AbortError');

      expect(fetch.mock.calls).length(1);
      expect(beforeRetry.mock.calls).length(0);
    });
  });

  test('tolerateFailure', async () => {
    let { client, fetch } = createClient(
      { tolerateFailure: true },
      () => new Response('', { status: 500 })
    );
    let result = await client({ url: '/abc', fetch });
    expect(result.status).eq(500);
  });

  test('tolerateFailure on specific error codes', async () => {
    let { client, fetch } = createClient(
      { tolerateFailure: [500] },
      () => new Response('', { status: 500 })
    );

    let result = await client({ url: '/abc', fetch });
    expect(result.status).eq(500);

    fetch.mockImplementationOnce(() => new Response('', { status: 404 }));
    result = await client({ url: '/abc', fetch }).catch((e) => e);
    expect(result).instanceOf(HttpError);
  });

  test('pass abort controller', async () => {
    let { client, fetch } = createClient({}, (_url, { signal }) => {
      return new Promise<Response>((resolve, reject) => {
        signal?.throwIfAborted();
        signal?.addEventListener('abort', () => {
          reject(signal.reason);
        });
      });
    });

    let abort = new AbortController();
    let promise = client({ url: '/abc', fetch, abort });
    promise.abort();
    let result = await promise.catch((e) => e);

    expect(result.name).eq('AbortError');
    expect(abort.signal.aborted).eq(true);
  });

  test('implicit AbortController', async () => {
    let { client, fetch } = createClient({}, (_url, { signal }) => {
      return new Promise<Response>((resolve, reject) => {
        signal?.throwIfAborted();
        signal?.addEventListener('abort', () => {
          reject(signal.reason);
        });
      });
    });

    let promise = client({ url: '/abc', fetch });
    promise.abort();
    let result = await promise.catch((e) => e);

    expect(result.name).eq('AbortError');
  });

  test('json body', async () => {
    let { client, fetch } = createClient();
    let result = await client({
      url: '/abc',
      method: 'POST',
      fetch,
      json: { foo: 'bar' },
    });
    expect(result.status).eq(200);

    let req = fetch.mock.calls[0][1];
    expect(req.headers?.get('Content-Type')).eq('application/json');
    expect(req.body).eq(JSON.stringify({ foo: 'bar' }));
  });

  test('form data body', async () => {
    let { client, fetch } = createClient({}, () => new Response('{}', { status: 200 }));
    let body = new FormData();
    body.set('foo', 'bar');
    body.set('apple', 'orange');
    let result = await client({
      url: '/abc',
      method: 'POST',
      fetch,
      body,
    });
    expect(result.status).eq(200);

    let req = fetch.mock.calls[0][1];
    expect(req.headers?.get('Content-Type')).eq('multipart/form-data');
    expect(req.body).instanceOf(FormData);
  });

  test('URLSearchParams body', async () => {
    let { client, fetch } = createClient({}, () => new Response('{}', { status: 200 }));
    let body = new URLSearchParams();
    body.set('foo', 'bar');
    body.set('apple', 'orange');
    let result = await client({
      url: '/abc',
      method: 'POST',
      fetch,
      body,
    });
    expect(result.status).eq(200);

    let req = fetch.mock.calls[0][1];
    expect(req.headers?.get('Content-Type')).eq('application/x-www-form-urlencoded');
    expect(req.body).instanceOf(URLSearchParams);
  });

  test('pass in content-type', async () => {
    let { client, fetch } = createClient({}, () => new Response('{}', { status: 200 }));
    let body = new URLSearchParams();
    body.set('foo', 'bar');
    body.set('apple', 'orange');
    let result = await client({
      url: '/abc',
      method: 'POST',
      headers: {
        'content-type': 'text/plain',
      },
      fetch,
      body,
    });
    expect(result.status).eq(200);

    let req = fetch.mock.calls[0][1];
    expect(req.headers?.get('Content-Type')).eq('text/plain');
  });

  test('call json to extract JSON', async () => {
    let { client, fetch } = createClient({}, () => new Response('{"foo":"bar"}', { status: 200 }));
    let result = await client({
      url: '/abc',
      fetch,
    }).json();

    expect(result).toEqual({ foo: 'bar' });
  });

  test('call text to extract text', async () => {
    let { client, fetch } = createClient({}, () => new Response('{"foo":"bar"}', { status: 200 }));
    let result = await client({
      url: '/abc',
      fetch,
    }).text();

    expect(result).toEqual('{"foo":"bar"}');
  });

  describe('hooks', () => {
    test('beforeRequest modifies Request', async () => {
      let { client, fetch } = createClient(
        {
          hooks: {
            beforeRequest: [
              (request) => {
                request.headers.set('foo', 'bar');
              },
            ],
          },
        },
        () => new Response('{}', { status: 200 })
      );

      let result = await client({
        url: '/abc',
        fetch,
      });

      const called = fetch.mock.calls[0][1];
      expect(called.headers?.get('foo')).toEqual('bar');
    });

    test('beforeRequest returns new RequestInput', async () => {
      let { client, fetch } = createClient(
        {
          hooks: {
            beforeRequest: [
              (request) => {
                return {
                  url: '/def',
                };
              },
            ],
          },
        },
        () => new Response('{}', { status: 200 })
      );

      await client({
        url: '/abc',
        fetch,
      });

      const called = fetch.mock.calls[0][1];
      expect(called).toEqual({ url: '/def' });
    });

    test('beforeRequest returns new Request', async () => {
      let { client, fetch } = createClient(
        {
          hooks: {
            beforeRequest: [
              (request) => {
                return new Request('https://example.com');
              },
            ],
          },
        },
        () => new Response('{}', { status: 200 })
      );

      await client({
        url: '/abc',
        fetch,
      });

      const called = fetch.mock.calls[0][0];
      expect(called).instanceOf(Request);
      expect(called.url).toEqual('https://example.com/');
    });

    test('beforeRequest returns Response', async () => {
      let { client, fetch } = createClient(
        {
          hooks: {
            beforeRequest: [
              (request) => {
                return new Response('Ok', { status: 202 });
              },
            ],
          },
        },
        () => new Response('{}', { status: 200 })
      );

      let result = await client({
        url: '/abc',
        fetch,
      });

      expect(fetch).not.toHaveBeenCalled();
      expect(result.status).toEqual(202);
    });

    test('beforeRetry not called when no retries occur', async () => {
      let beforeRetry = vi.fn();
      let { client, fetch } = createClient({ hooks: { beforeRetry: [beforeRetry] } });

      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let response = await promise;
      expect(response.status).eq(200);

      expect(fetch.mock.calls).length(1);
      expect(beforeRetry.mock.calls).length(0);
    });

    test('beforeRetry alters Request', async () => {
      let { client, fetch } = createClient({
        hooks: {
          beforeRetry: [
            ({ request }) => {
              request.url = '/def';
            },
          ],
        },
      });
      fetch.mockImplementationOnce(() => new Response('{}', { status: 500 }));

      let promise = client({ url: '/abc', fetch }).catch((e) => e);
      await vi.advanceTimersByTimeAsync(50000);
      let response = await promise;
      expect(response.status).eq(200);

      expect(fetch.mock.calls).length(2);
      expect(fetch.mock.calls[0][0]).eq('/abc');
      expect(fetch.mock.calls[1][0]).eq('/def');
    });

    test('beforeError', async () => {
      vi.useFakeTimers();

      try {
        let beforeError = vi.fn();
        let { client, fetch } = createClient(
          { hooks: { beforeError: [beforeError] } },
          () => new Response('{}', { status: 500 })
        );

        let promise = client({ url: '/abc', fetch }).catch((e) => e);
        await vi.advanceTimersByTimeAsync(50000);
        await promise;
        expect(beforeError.mock.calls).length(1);
        expect(beforeError.mock.calls[0][0]).instanceOf(HttpError);
      } finally {
        vi.useRealTimers();
      }
    });

    test('afterResponse', async () => {
      let afterResponse = vi.fn();
      let { client, fetch } = createClient({ hooks: { afterResponse: [afterResponse] } });
      await client({ url: '/abc', fetch });

      expect(afterResponse.mock.calls).length(1);
      expect(afterResponse.mock.calls[0][0].url).toEqual('/abc');
      expect(afterResponse.mock.calls[0][2]).instanceof(Response);
    });

    test('afterResponse returns new Response', async () => {
      let { client, fetch } = createClient({
        hooks: { afterResponse: [() => new Response('Ok', { status: 202 })] },
      });
      let response = await client({ url: '/abc', fetch });

      expect(response.status).toEqual(202);
    });
  });
});
