import { mount } from "svelte";
import { Widget } from "@seelen-ui/lib";
import App from "./app.svelte";
import { loadTranslations } from "./i18n/index.ts";

import "@seelen-ui/lib/styles/reset.css";

await loadTranslations();

await Widget.self.init({ saveAndRestoreLastRect: false });
await Widget.self.window.setResizable(false);
await Widget.self.window.setIgnoreCursorEvents(true);

const root = document.getElementById("root")!;
mount(App, { target: root });
