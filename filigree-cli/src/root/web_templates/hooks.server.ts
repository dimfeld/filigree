import { getUser } from '$lib/server/user.js';
import type { Handle } from '@sveltejs/kit';
import { sequence } from '@sveltejs/kit/hooks';

const auth: Handle = async ({ event, resolve }) => {
  event.locals.user = await getUser(event);
  return resolve(event);
};

export const handle = sequence(auth);

export function handleError({ error, event, message, status }) {
  console.dir(error);
  return {
    status,
    message,
    error: error.stack ?? JSON.stringify(error, null, 2),
  };
}

