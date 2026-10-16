// Copies the data files of pdf.js (the maps of characters and the
// standard fonts) into ui/public/pdfjs, so that they are built into
// the program with the page: the viewer of standards (step 18 of
// docs/todo.md) never loads anything from the network.
import { cpSync, mkdirSync, rmSync } from "node:fs";

const from = "node_modules/pdfjs-dist";
const to = "ui/public/pdfjs";

rmSync(to, { recursive: true, force: true });
mkdirSync(to, { recursive: true });

for (const folder of ["cmaps", "standard_fonts"]) {
  cpSync(`${from}/${folder}`, `${to}/${folder}`, { recursive: true });
}
