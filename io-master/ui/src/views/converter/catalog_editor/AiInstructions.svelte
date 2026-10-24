<!--
  The element "instructions to the AI" of the editor of a catalog
  (its commands in src/views/converter/catalog_editor/catalog_editor.rs): a dialog with the
  instructions of the task that proposes the groups or the controls.
  Saving an empty text brings back the default ones.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../../lib/api/errors";
  import * as catalogEditor from "../../../lib/api/converter/catalog_editor";

  let { which, onclose }: { which: catalogEditor.Which; onclose: () => void } = $props();

  let text = $state("");
  let problem = $state("");
  let notice = $state("");

  onMount(async () => {
    try {
      text = await catalogEditor.aiInstructionsGet(which);
    } catch (error) {
      problem = sentence(error);
    }
  });

  async function save(written: string) {
    try {
      text = await catalogEditor.aiInstructionsSave(which, written);
      notice = "The instructions were saved.";
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="overlay" role="presentation">
  <div class="dialog wide">
    <h2>Instructions to the AI: {which === "catalog_outline" ? "groups" : "controls"}</h2>
    <p class="note">Evaluation support, kept in your database. Saving an empty text brings back the default ones.</p>
    {#if problem}<p class="problem">{problem}</p>{/if}
    {#if notice}<p class="done">{notice}</p>{/if}
    <textarea class="mono" rows="18" bind:value={text}></textarea>
    <div class="buttons">
      <button type="button" onclick={() => save(text)}>Save</button>
      <button type="button" onclick={() => save("")}>Default</button>
      <button type="button" onclick={onclose}>Close</button>
    </div>
  </div>
</div>
