import { createApp } from "vue";
import "./style.css";
import "vue-sonner/style.css";
import { initTheme } from "./lib/theme";
import App from "./App.vue";

initTheme();
createApp(App).mount("#app");
