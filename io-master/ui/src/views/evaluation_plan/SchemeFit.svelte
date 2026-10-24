<!--
  Fits the drawing of a task scheme (TaskScheme.svelte) to its canvas when
  the number of blocks changes, so that a block just added is seen. It lives
  inside <SvelteFlow>, whose context useSvelteFlow needs; only presentation,
  nothing is sent to the program.
-->
<script lang="ts">
  import { useSvelteFlow } from "@xyflow/svelte";

  let { count }: { count: number } = $props();

  const flow = useSvelteFlow();
  let seen = 0;

  $effect(() => {
    // The first drawing is fitted by the prop fitView of <SvelteFlow>.
    if (seen !== 0 && seen !== count) {
      // The new block is measured in the next frame, then the view fits it.
      requestAnimationFrame(() => flow.fitView({ maxZoom: 1, duration: 200 }));
    }

    seen = count;
  });
</script>
