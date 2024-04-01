import type { Handle } from '@sveltejs/kit';
import { client } from 'filigree-web';

/** Fetch info for the current user. This places the user at `event.locals.user`,
 * which allows subsequent calls in the same request to use the first call's result. */
export const getUser: Handle = async ({ event, resolve }) => {
  if (!event.cookies.get('sid')) {
    return resolve(event);
  }

  const response = await client({
    url: '/api/self',
    fetch: event.fetch,
    // 401 means user is not logged in
    tolerateFailure: [401],
  });

  if (response.status === 200) {
    event.locals.user = await response.json();
  }

  return resolve(event);
};

