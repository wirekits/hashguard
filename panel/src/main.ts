import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// In a plain browser during development, fake the Rust backend.
// Vite drops this branch from production builds.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  const { installMocks } = await import("./lib/mock");
  installMocks();
}

mount(App, { target: document.getElementById("app")! });
