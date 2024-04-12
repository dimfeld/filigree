<script>
  import { ThemeInit, ThemeSwitch, settings } from 'svelte-ux';
  import '../app.postcss';

  let { data } = $props();

  settings({
    components: {
      Field: {
        labelPlacement: 'top',
      },
      TextField: {
        labelPlacement: 'top',
      },
      Menu: {
        transitionParams: { duration: 150 }
      }
    }
  });

</script>

<!-- inject code into <head> to load theme before client-side Svelte JS runs -->
<ThemeInit />

<div id="top" class="h-full min-h-screen w-full overflow-auto bg-surface-100 text-surface-content">
  <nav class="flex h-8 w-full items-center justify-end gap-4 p-2 pt-4">
    <div>
    </div>
    <ThemeSwitch />
  </nav>
  <slot />
</div>
