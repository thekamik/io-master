<!--
  The element "connection line" of the view Chat
  (src/views/chat/connection_line/): which AI connection answers in the
  chat. It tells the view who writes the replies (onread).
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../lib/api/errors";
  import * as connectionLine from "../../lib/api/chat/connection_line";

  let { onread }: { onread: (line: connectionLine.ConnectionLineView | null) => void } = $props();

  let line: connectionLine.ConnectionLineView | null = $state(null);
  /** Whether the line was read (to tell "none" from "not yet"). */
  let read = $state(false);
  let problem = $state("");

  onMount(async () => {
    try {
      line = await connectionLine.get();
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
