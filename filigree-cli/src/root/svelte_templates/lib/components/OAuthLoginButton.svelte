<script lang="ts">
  import { goto, invalidateAll } from '$app/navigation';
  import type { Snippet } from 'svelte';
  import { Button } from 'svelte-ux';

  const { provider, name, onMessage, children } = $props<{
    onMessage?: (message: string) => void;
    provider: string;
    name: string;
    children?: Snippet;
  }>();

  let loginUrl = $derived(`/api/auth/oauth/login/${provider}`);
  let message = $state('');

  function oauthLogin(e: SubmitEvent) {
    e.preventDefault();
    const loginWindow = window.open(
      `${loginUrl}?frompopup=true`,
      'oauthLogin',
      'width=600,height=400'
    );

    if (loginWindow) {
      window.addEventListener('message', function handler(event) {
        loginWindow.close();
        window.removeEventListener('message', handler);

        let data = event.data;
        if (data.success) {
          invalidateAll();
          goto(data.redirectTo || '/');
        } else if (data.error) {
          message = 'message' in data.error ? data.error.message : data.error;
        } else {
          message = JSON.stringify(data);
        }

        onMessage?.(message);
      });
    } else {
      goto(loginUrl);
    }
  }
</script>

<form action={loginUrl} method="GET" onsubmit={oauthLogin}>
  {#if children}
    {@render children()}
  {:else}
    <Button color="primary" variant="fill-outline" class="w-full" rounded type="submit"
      >Login with {name}</Button
    >
  {/if}
</form>
