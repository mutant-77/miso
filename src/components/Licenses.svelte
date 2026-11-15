<script lang="ts">
  import type { License } from '../lib/ipc/types/License'
  import { getLicenses } from '../lib/ipc/commands'

  let licenses = $state<License[]>([])
  getLicenses().then(
    (all) => (licenses = all),
    (err) => console.error(err),
  )
</script>

<div class="licencias">
  {#each licenses as { title, kind, body } (title)}
    <article>
      <h2>{title}</h2>
      <p class="tipo">{kind}</p>
      <pre>{body}</pre>
    </article>
  {/each}
</div>

<style>
  .licencias {
    display: flex;
    flex-direction: column;
    gap: 24px;
    min-width: 0;
    padding: 12px 20px 20px;
    user-select: text;
  }

  h2 {
    margin: 0;
    font: 600 14px 'Segoe UI Variable Text', 'Segoe UI', sans-serif;
  }

  .tipo {
    margin: 2px 0 8px;
    font-size: 12px;
    opacity: 0.7;
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font: 11px Consolas, 'Cascadia Mono', monospace;
  }
</style>
