import { RequestEvent } from '@sveltejs/kit';

export type ProtectRoutesResult = 'ok' | 'unknown-user' | 'forbidden';

/** Route matching configuration. A path with a trailing `/` will match only on subroutes of that path. Routes
 * without a trailing slash will match both the exact route and the paths under that route.
 *
 * `/posts/` will match `/posts/1` but not `/posts`,
 * `/posts` will match both `/posts/1` and `/posts`
 */
export interface ProtectRoutesConfig<USER> {
  /** Routes to allow unauthenticated users to access. If this is set, all other routes will require the user to be
   * logged in. */
  allowUnauthed?: string[];

  /** Routes that require the user to be logged in, but no particular permission.
   * Routes covered by `check` below can be omitted. */
  requireAuth?: string[];

  /** Routes and permissions required by them */
  check?: Record<string, (user: USER) => boolean>;
}

function escapeRegExp(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function processRouteList(input: string[] | undefined, captureGroups: boolean): RegExp | null {
  if (!input?.length) {
    return null;
  }

  let components: string[] = [];

  for (let route of input) {
    let prefixOnly = route.endsWith('/');
    route = escapeRegExp(route);

    if (captureGroups) {
      route = `(${route})`;
    }

    if (prefixOnly) {
      // Match the prefix
      components.push(`^${route}`);
    } else {
      // Match the exact page
      components.push(`^${route}$`);
      // Match as a route prefix
      components.push(`^${route}/`);
    }
  }

  // Sort longest matches first so that the regex matches as expected
  components.sort((a, b) => b.length - a.length);

  return new RegExp(components.join('|'), 'i');
}

/** Return a matcher for `ProtectRoutesConfig#check` that checks for all of the given permissions. */
export function hasPermissions(perms: string[]): (user: { permissions: string[] }) => boolean {
  return (user: { permissions: string[] }) => perms.every((p) => user.permissions.includes(p));
}

export function protectRoutes<USER>(config: ProtectRoutesConfig<USER>) {
  let unauthed = processRouteList(config.allowUnauthed, false);
  let authed = processRouteList(config.requireAuth, false);

  let checkMatch = config.check ? processRouteList(Object.keys(config.check), true) : null;

  return (event: RequestEvent): ProtectRoutesResult => {
    let path = event.route.id ?? event.url.pathname;

    if (!event.locals.user) {
      if (unauthed && !unauthed.test(path)) {
        return 'unknown-user';
      }

      if (authed && authed.test(path)) {
        return 'unknown-user';
      }
    }

    let match = checkMatch?.exec(path);
    if (match && config.check) {
      let key = match.slice(1).find((m) => !!m);
      let checker = config.check[key];
      if (event.locals.user && checker(event.locals.user)) {
        return 'ok';
      } else {
        return event.locals.user ? 'forbidden' : 'unknown-user';
      }
    }

    return 'ok';
  };
}
