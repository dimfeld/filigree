import { expect, test, describe, beforeEach, afterEach, vi } from 'vitest';
import {
  mergeRetryOptions,
  updateHeaders,
  type ClientOptions,
  makeClient,
  HttpError,
  TimeoutError,
  RateLimitError,
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
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function createClient(clientOptions: ClientOptions, response?: () => Response) {
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
  });

  describe('retry', () => {
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

  test.skip('tolerateFailure');
  test.skip('tolerateFailure on specific error codes');
  test.skip('pass abort controller');
  test.skip('call abort');
  test.skip('json body');
  test.skip('form data body');
  test.skip('URLSearchParams body');
  test.skip('pass in content-type');
  test.skip('call json to extract JSON');
  test.skip('call text to extract text');
  describe('hooks', () => {
    test.skip('beforeRequest modifies Request');
    test.skip('beforeRequest returns new Request');
    test.skip('beforeRequest returns Response');

    test.skip('beforeRetry not called when no retries occur');
    test.skip('beforeRetry alters Request');
    test.skip('beforeError');
    test.skip('afterResponse');
    test.skip('afterResponse returns new Response');
  });
});
