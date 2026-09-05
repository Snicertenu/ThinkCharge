import { getCurrentWindow } from "@tauri-apps/api/window";
import { SettingsView } from "./components/SettingsView";
import { WidgetView } from "./components/WidgetView";

function isSettingsView(): boolean {
  const params = new URLSearchParams(globalThis.location.search);
  if (params.get("view") === "settings") {
    return true;
  }
  return getCurrentWindow().label === "settings";
}

function App() {
  if (isSettingsView()) {
    return <SettingsView />;
  }

  return <WidgetView />;
}

export default App;
