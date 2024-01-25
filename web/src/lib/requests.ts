import type { HttpMethod } from './client.js';

export function forwardTo(method: HttpMethod, url: string, request: Request, body?: BodyInit) {
  let newRequest = fetch(url, {
    method,
    body,
    headers: request.headers,
    signal: request.signal,
    credentials: 'include',
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
