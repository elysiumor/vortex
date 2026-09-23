import { createApp } from "vue";
import "./style.css";
import "vue-sonner/style.css";
import { initTheme } from "./lib/theme";
import { installDiagnostics } from "./lib/diag";
import App from "./App.vue";

initTheme();
installDiagnostics();
createApp(App).mount("#app");
