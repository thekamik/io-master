<!--
  The view Knowledge (RAG) (step 57 of docs/todo.md; its commands in
  src/views/knowledge/knowledge.rs): the knowledge bases of the AI. The list
  of bases (KnowledgeList) or one base (KnowledgeBase) with its sources and
  its search (KnowledgeSearch). Presentation only.
-->
<script lang="ts">
  import KnowledgeBase from "./KnowledgeBase.svelte";
  import KnowledgeList from "./KnowledgeList.svelte";

  /** The base open, if any. */
  let base: number | null = $state(null);
</script>

{#if base !== null}
  <KnowledgeBase id={base} onclose={() => (base = null)} />
{:else}
  <KnowledgeList onopen={(id) => (base = id)} />
{/if}
