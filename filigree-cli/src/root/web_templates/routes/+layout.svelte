<script>
  import { ThemeInit, ThemeSwitch, settings } from 'svelte-ux';
  import { page } from '$app/stores';
  import '../app.postcss';

  let { data } = $props();

  settings({});
</script>

<!-- inject code into <head> to load theme before client-side Svelte JS runs -->
<ThemeInit />

<div id="top" class="h-full min-h-screen w-full overflow-auto bg-surface-100 text-surface-content">
  <nav class="flex h-8 w-full items-center justify-end gap-4 p-2 pt-4">
    <div>
      {#if data.user}
        Logged in as {data.user.name}
      {:else if showLogin}
        <a href="/login">Log in</a>
      {/if}
    </div>
    <ThemeSwitch />
  </nav>
  <slot />
</div>
