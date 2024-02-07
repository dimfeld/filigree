import { error, fail, type RequestEvent } from '@sveltejs/kit';
import { client, type Client, type HttpMethod, type RequestOptions } from './client.js';

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
  return thisClient({
    url: '/api/' + url,
    method,
    headers: event.request.headers,
    signal: event.request.signal,
    body: event.request.body,
    fetch: event.fetch,
    ...options,
  });
}

export interface ErrorResponse<KIND extends string = string, DETAILS extends object = object> {
  error: {
    kind: KIND;
    message: string;
    details: DETAILS;
  };
}

export function isErrorResponse<T extends ErrorResponse<string, object>>(
  obj: object | undefined,
  kind: string
): obj is T {
  if (!obj) {
    return false;
  }
  return obj && 'error' in obj && (obj.error as T['error'])?.kind === kind;
}

export async function forwardFormToApi(
  method: HttpMethod,
  url: string,
  event: RequestEvent,
  options?: Partial<RequestOptions> & { client?: Client }
) {
  const response = await forwardToApi(method, url, event, options);

  if (response.status === 400) {
    // TODO return body
    fail(400, {});
  } else if (!response.ok) {
    // TODO return body
    error(response.status, {});
  }

  return {
    // todo
  };
}

/** Copy the Set-Cookie headers from the given response into the given headers. */
export function copyCookies(response: Response, headers?: HeadersInit): Headers {
  let h = headers instanceof Headers ? headers : new Headers(headers);
  let cookies = response.headers.getSetCookie();
  for (let cookie of cookies) {
    h.append('Set-Cookie', cookie);
  }
  return h;
}
