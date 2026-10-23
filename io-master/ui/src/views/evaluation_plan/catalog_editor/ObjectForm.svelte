<!--
  The element "object" of the editor of a catalog
  (src/views/evaluation_plan/catalog_editor/object_form/): the group or control chosen,
  its parts as the standard reads, and its own fields as JSON of OSCAL,
  changed and saved here (what it holds stays). Its quotes come in
  between (children, the element "quotes"). A change tells the view
  (onchanged, with the id the object has now); "+ ... in it" only tells
  the view (onadd).
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  import { sentence } from "../../../lib/api/errors";
  import * as objectForm from "../../../lib/api/evaluation_plan/catalog_editor/object_form";
  import type { Place } from "../../../lib/api/evaluation_plan/catalog_editor/new_object";
  import PartRows from "../../../components/part_rows/PartRows.svelte";

  let {
    uuid,
    id,
    version,
    onchanged,
    onadd,
    children,
  }: {
    uuid: string;
    id: string;
    version: number;
    onchanged: (id: string | null) => void;
    onadd: (kind: "group" | "control", place: Place) => void;
    children: Snippet;
  } = $props();

  let object: objectForm.ObjectView | null = $state(null);
  let fields = $state("");
  let problem = $state("");
  let notice = $state("");

  $effect(() => {
    void version;
    load(uuid, id);
  });

  async function load(catalog: string, chosen: string) {
    try {
      object = await objectForm.get(catalog, chosen);
      fields = JSON.stringify(object.fields, null, 2);
      problem = "";
    } catch (error) {
      object = null;
      problem = sentence(error);
    }
  }

  async function save() {
    if (!object) {
      return;
    }

    notice = problem = "";

    try {
      const changed = JSON.parse(fields) as { id?: string };

      if (object.kind === "group") {
        await objectForm.saveGroup(uuid, id, changed);
      } else {
        await objectForm.saveControl(uuid, id, changed);
      }

      notice = "Saved.";
      onchanged(changed.id ?? id);
    } catch (error) {
      problem = error instanceof SyntaxError ? `The text is not JSON: ${error.message}` : sentence(error);
    }
  }

  async function remove() {
    try {
      await objectForm.remove(uuid, id);
      onchanged(null);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="card catalog-object">
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if notice}<p class="done">{notice}</p>{/if}

  {#if object}
    <div class="panel-head">
      <h3>{object.kind === "group" ? "Group" : "Control"} {object.id}</h3>
      <div class="buttons">
        {#if object.kind === "group"}
          <button type="button" class="small" onclick={() => onadd("group", { kind: "group", id })}>+ Group in it</button>
          <button type="button" class="small" onclick={() => onadd("control", { kind: "group", id })}>+ Control in it</button>
        {:else}
          <button type="button" class="small" onclick={() => onadd("control", { kind: "control", id })}>+ Enhancement</button>
        {/if}
        <button type="button" class="small danger" onclick={remove}>Delete with its content</button>
      </div>
    </div>
    <h4>{object.title}</h4>
    <PartRows rows={object.parts} />

    {@render children()}

    <details class="settings">
      <summary>Its fields in OSCAL</summary>
      <textarea class="mono" rows="14" bind:value={fields}></textarea>
      <p class="note">
        The own fields of the object (JSON of OSCAL 1.2.3); the groups and controls it holds stay as they are. Every
        change is checked against the schema of NIST.
      </p>
      <div class="buttons"><button type="button" onclick={save}>Save</button></div>
    </details>
  {/if}
</div>
