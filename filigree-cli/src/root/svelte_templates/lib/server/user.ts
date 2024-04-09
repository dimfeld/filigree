import type { RequestEvent } from '@sveltejs/kit';
import type { SelfUser } from '$lib/api_types.js';
import { client } from 'filigree-svelte';

/** A SvelteKit server hook that fetches info for the current user and places it at event.locals.user. */
export async function getUser({ fetch, cookies }: RequestEvent): Promise<SelfUser | null> {
  if (!cookies.get('sid')) {
    return null;
  }

  const response = await client({
    url: '/api/self',
    fetch,
    // 401 means user is not logged in
    tolerateFailure: [401],
  });

  if (response.status === 200) {
    return (await response.json()) as SelfUser;
  } else {
    return null;
  }
}

