<script lang="ts">
  import { t } from '../lib/i18n.svelte'
</script>

<div class="empty">
  <p>{t('empty.open')}</p>
</div>

<style>
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    opacity: 0.6;
  }

  p {
    margin: 0;
  }
</style>
