export async function handle({ event, resolve }) {
  return resolve(event);
}

export function handleError({ error, event, message, status }) {
  console.dir(error);
  return {
    status,
    message,
    error: error.stack ?? JSON.stringify(error, null, 2),
  };
}
