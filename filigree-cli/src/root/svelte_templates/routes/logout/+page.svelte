<script lang="ts">
  import { browser } from '$app/environment';
  import { invalidateAll } from '$app/navigation';

  if (browser) {
    invalidateAll();
  }
</script>

<div class="grid justify-items-center">
  <p>You have been logged out.</p>
  <p><a href="/">Return to the app</a></p>
</div>
