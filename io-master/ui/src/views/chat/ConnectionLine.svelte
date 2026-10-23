<!--
  The element "connection line" of the view Chat
  (its command connection_line_get in src/views/chat/chat.rs): which AI connection answers in the
  chat. It tells the view who writes the replies (onread).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as chat from "../../lib/api/chat";

  let { onread }: { onread: (line: chat.ConnectionLineView | null) => void } = $props();

  let line: chat.ConnectionLineView | null = $state(null);
  /** Whether the line was read (to tell "none" from "not yet"). */
  let read = $state(false);
  let problem = $state("");

  onMount(async () => {
    try {
      line = await chat.connectionLineGet();
      read = true;
      onread(line);
    } catch (error) {
      problem = `The AI connection cannot be read: ${sentence(error)}`;
    }
  });
</script>

{#if problem}
  <p class="problem">{problem}</p>
{:else if read && line}
  <p class="status" title="Another connection is chosen in Settings, tab AI">
    <strong>{line.name}</strong> · {line.writer}
  </p>
{:else if read}
  <p class="problem">No AI connection is chosen. Set one up in Settings, tab AI, to use the chat.</p>
{/if}
