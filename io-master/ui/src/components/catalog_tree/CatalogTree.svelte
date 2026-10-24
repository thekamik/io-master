<!--
  The component "tree of a catalog" (src/components/catalog_tree/, its
  command catalog_tree_get): the groups and controls, as the rows the program
  gives with their level, drawn without recursion. Used by the editor of a
  catalog (Converter), with "+ Group" and "+ Control" (onadd), and by the
  browser of the standards (Evaluation plan), without them. Choosing an
  object only tells the view. It reads the tree again when the view says the
  catalog changed (version).
-->
<script lang="ts">
  import { sentence } from "../../lib/api/errors";
  import * as catalogTree from "../../lib/api/components/catalog_tree";

  let {
    uuid,
    version,
    selected,
    onchoose,
    onread,
    onadd,
  }: {
    uuid: string;
    version: number;
    selected: string | null;
    onchoose: (id: string, kind: "group" | "control") => void;
    onread: (tree: catalogTree.CatalogTreeView) => void;
    onadd?: (kind: "group" | "control") => void;
  } = $props();

  let tree: catalogTree.CatalogTreeView | null = $state(null);
  let problem = $state("");

  $effect(() => {
    void version;
    load(uuid);
  });

  async function load(catalog: string) {
    try {
      tree = await catalogTree.catalogTreeGet(catalog);
      problem = "";
      onread(tree);
    } catch (error) {
      problem = sentence(error);
    }
  }
</script>

<div class="card catalog-side">
  {#if onadd}
    <div class="buttons">
      <button type="button" class="small" onclick={() => onadd("group")}>+ Group</button>
      <button type="button" class="small" onclick={() => onadd("control")}>+ Control</button>
    </div>
  {/if}
  {#if problem}<p class="problem">{problem}</p>{/if}
  {#if tree && tree.rows.length === 0}
    <p class="note">The catalog is empty.</p>
  {/if}
  {#if tree}
    <ul class="catalog-tree">
      {#each tree.rows as row, index (row.id ?? index)}
        <li class:active={row.id !== null && selected === row.id} style="padding-left: {row.level * 0.9}rem">
          <button
            type="button"
            class="link"
            class:group={row.kind === "group"}
            onclick={() => row.id && onchoose(row.id, row.kind)}
          >
            <span class="object-id">{row.id ?? ""}</span> {row.title}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
