import { getUser } from '$lib/server/user.js';
import { type Handle, error, redirect, type HandleFetch } from '@sveltejs/kit';
import { hasPermissions, protectRoutes } from 'filigree-web/auth/routes';
import { sequence } from '@sveltejs/kit/hooks';

const protect = protectRoutes({
  allowUnauthed: ['/login', '/forgot', '/auth'],
  // requireAuth: [],
  check: {
    '/organization/admin': hasPermissions(['org_admin']),
    '/admin': hasPermissions(['_global:admin']),
  },
});

const auth: Handle = async ({ event, resolve }) => {
  if (event.url.pathname.startsWith('/api')) {
    // API handles its own auth, and we don't want to call getUser when doing an API call since it results in
    // an infinite loop.
    return resolve(event);
  }

  event.locals.user = await getUser(event);

  const protectResult = protect(event);
  if (protectResult === 'unknown-user') {
    let qs = new URLSearchParams({
      redirectTo: event.url.pathname,
    });
    redirect(302, '/login?' + qs.toString());
  } else if (protectResult === 'forbidden') {
    error(403);
  }

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

