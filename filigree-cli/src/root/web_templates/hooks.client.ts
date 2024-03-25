export function handleError({ error, event, message, status }) {
  console.dir(error);
  return {
    status,
    message,
    error,
  };
}
