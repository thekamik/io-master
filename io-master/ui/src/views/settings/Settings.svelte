<!--
  The settings of the AI (their commands in src/panels/views/settings/):
  only the model Integration (user, 2026-10-29): the AI services the program
  asks, their keys, the active one and the check of the connection. The
  agents, their knowledge and their journals have a panel of their own.
-->
<script lang="ts">
  import IntegrationList from "./IntegrationList.svelte";
</script>

<div class="page-head">
  <h1>AI settings</h1>
  <p>The AI services the program asks: their addresses, models and keys.</p>
</div>

<IntegrationList />
