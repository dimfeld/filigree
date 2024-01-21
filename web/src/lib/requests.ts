import type {HttpMethod} from "./client.js";

export function forwardTo(method: HttpMethod, url: string, request: Request, body?: BodyInit) {
  let newRequest = fetch(url, {
    method,
    body,
    headers: request.headers,
    signal: request.signal,
    credentials: 'include',

  })
}
