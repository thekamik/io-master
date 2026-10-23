<!--
  The element "new object" of the editor of a catalog
  (its commands in src/views/evaluation_plan/catalog_editor/catalog_editor.rs): a dialog that adds a group or a
  control, written as JSON of OSCAL, at a place of the catalog. When it is
  added it tells the view (ondone, with its id).
-->
<script lang="ts">
  import { sentence } from "../../../lib/api/errors";
  import * as catalogEditor from "../../../lib/api/evaluation_plan/catalog_editor";

  let {
    uuid,
    kind,
    place,
    ondone,
    onclose,
  }: {
    uuid: string;
    kind: "group" | "control";
    place: catalogEditor.Place;
    ondone: (id: string | null) => void;
    onclose: () => void;
  } = $props();

  // The dialog is made anew for every object, so the kind it was opened
  // with is the one it keeps.
  // svelte-ignore state_referenced_locally
  const template = kind === "group" ? { id: "", title: "" } : { id: "", title: "", parts: [{ name: "statement", prose: "" }] };

  let text = $state(JSON.stringify(template, null, 2));
  let problem = $state("");

  async function add(event: SubmitEvent) {
    event.preventDefault();

    try {
      const object = JSON.parse(text) as { id?: string };

      if (kind === "group") {
        await catalogEditor.newObjectAddGroup(uuid, place, object);
      } else {
        await catalogEditor.newObjectAddControl(uuid, place, object);
      }

      ondone(object.id ?? null);
    } catch (error) {
      problem = error instanceof SyntaxError ? `The text is not JSON: ${error.message}` : sentence(error);
    }
  }
</script>

<div class="overlay" role="presentation">
  <form class="dialog" onsubmit={add}>
    <h2>A new {kind}</h2>
    <p class="note">
      In {place.kind === "top" ? "the top of the catalog" : place.id}. JSON of OSCAL 1.2.3: an id (for CC, "FAU_GEN.1"),
      a title, and for a control its statement.
    </p>
    {#if problem}<p class="problem">{problem}</p>{/if}
    <textarea class="mono" rows="12" bind:value={text}></textarea>
    <div class="buttons">
      <button type="submit">Add</button>
      <button type="button" onclick={onclose}>Cancel</button>
    </div>
  </form>
</div>
