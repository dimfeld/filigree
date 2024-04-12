<script lang="ts">
  import { browser } from '$app/environment';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/stores';
  import OAuthLoginButton from '$lib/components/OAuthLoginButton.svelte';
  import { manageForm, LoginFormSchema } from 'filigree-svelte';
  import { Button, TextField } from 'svelte-ux';

  const { data, form } = $props();

  let topMessage = $state(data.message);
  $effect(() => {
    topMessage = data.message;
  });

  let formManager = manageForm({
    schema: LoginFormSchema,
    form,
    onSubmit() {
      topMessage = '';
    },
    onSuccess(result) {
      // Redirect means we logged in with a password, so we want to fetch the user again.
      // Normal success means passwordless login, and so we aren't actually logged in yet.
      return {
        resetForm: false,
        invalidateAll: result.type === 'redirect',
      };
    },
  });

  let { errors, fieldErrors, message, formData } = $derived(formManager);
  let { enhance } = formManager;
  function handleMessage(m: string) {
    formManager.message = m;
  }

  if (browser && data.logInSuccess) {
    invalidateAll();
    setTimeout(() => {
      goto(data.redirect_to || '/');
    }, 3000);
  }

  let showMessage = $derived(message || errors?.messages?.[0]);
</script>

<div class="mx-auto mt-8 w-full max-w-lg flex flex-col gap-8">
  {#if topMessage}
    <p>{topMessage}</p>
  {/if}

  {#if data.logInSuccess}
    <p>You have been logged in. Redirecting to the app...</p>
  {:else}
    <form
      class="flex flex-col gap-4"
      method="POST"
      action={formData.password ? '?/login' : '?/passwordless'}
      use:enhance
    >
      <input
        type="hidden"
        name="redirect_to"
        value={$page.url.searchParams.get('redirectTo') || '/'}
      />
      <TextField
        labelPlacement="top"
        name="email"
        label="Email"
        bind:value={formData.email}
        error={fieldErrors.email}
      />
      <TextField
        labelPlacement="top"
        name="password"
        label="Password"
        type="password"
        error={fieldErrors.password}
        bind:value={formData.password}
      />
      <Button variant="fill" color="primary" type="submit">
        {#if formData.password}
          Login with Password
        {:else}
          Email me a Login Link
        {/if}
      </Button>
      <p class="text-sm">
        {#if showMessage}
          {showMessage}
        {:else}
          Type a password, or leave it blank to receive an email with a login link.
        {/if}
      </p>
    </form>

    <div class="flex w-full flex-col items-stretch gap-2">
      {#if data.oauthEnabled?.github}
        <OAuthLoginButton provider="github" name="GitHub" onMessage={handleMessage} />
      {/if}
      {#if data.oauthEnabled?.twitter}
        <OAuthLoginButton provider="twitter" name="Twitter" onMessage={handleMessage} />
      {/if}
      {#if data.oauthEnabled?.google}
        <OAuthLoginButton provider="google" name="Google" onMessage={handleMessage} />
      {/if}
    </div>
  {/if}
</div>
