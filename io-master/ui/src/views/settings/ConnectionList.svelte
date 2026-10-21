<!--
  The element "list of connections" of the view Settings, tab AI
  (src/views/settings/connection_list/). Choosing a connection and "New
  connection" only tell the view (onchoose). It tells the view which
  connection the chat uses (onread). The view asks it to read the list
  again after the form changed it (reload(), below).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as connectionList from "../../lib/api/settings/connection_list";

  let {
    chosen,
    busy,
    onchoose,
    onread,
  }: {
    chosen: number | "new" | null;
    busy: boolean;
    onchoose: (chosen: number | "new") => void;
    onread: (active: number | null) => void;
  } = $props();

  let list: connectionList.ConnectionListView | null = $state(null);
  let problem = $state("");

  /** Reads the connections again; called by the view after a change. */
  export async function reload() {
    try {
      list = await connectionList.get();
      problem = "";
      onread(list.active);
    } catch (error) {
      problem = `Your AI connections cannot be read: ${sentence(error)}`;
    }
  }

  onMount(reload);
</script>

<aside class="chat-list">
  <button class="primary" onclick={() => onchoose("new")} disabled={busy}>New connection</button>
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if list}
    <ul>
      {#each list.connections as connection (connection.id)}
        <li>
          <button class="link" class:active={connection.id === chosen} onclick={() => onchoose(connection.id)} disabled={busy}>
            {connection.name}
          </button>
          {#if connection.id === list.active}<span class="badge success">used by the chat</span>{/if}
        </li>
      {/each}
    </ul>
    {#if list.connections.length === 0}<p class="note">No connection yet.</p>{/if}
  {/if}
</aside>
