import { getUser } from '$lib/server/user.js';
export async function load(event) {
  return {
    user: await getUser(event),
  };
}
