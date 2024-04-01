import { getUser } from '$lib/server/user.js';

export function handle(event) {
  return getUser(event);
}

export function handleError({ error, event, message, status }) {
  console.dir(error);
  return {
    status,
    message,
    error: error.stack ?? JSON.stringify(error, null, 2),
  };
}

