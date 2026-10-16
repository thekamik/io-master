// The start of the page: mounts the application into #app.
//
// The application is imported dynamically, so that an error while its
// modules load (a library that needs a newer web view, for example)
// is shown in the window instead of leaving it empty.
import { mount } from "svelte";

import "./style.css";

const target = document.getElementById("app");

if (!target) {
  throw new Error("the element #app is missing from index.html");
}

import("./App.svelte")
  .then(({ default: App }) => mount(App, { target }))
  .catch((error: unknown) => {
    const message = document.createElement("p");

    message.className = "problem centered";
    message.textContent = `The window could not start: ${error instanceof Error ? `${error.name}: ${error.message}` : String(error)}`;

    // Where it happened, for a report of the fault.
    const stack = document.createElement("pre");

    stack.className = "note";
    stack.textContent = error instanceof Error ? (error.stack ?? "") : "";
    target.replaceChildren(message, stack);
  });
