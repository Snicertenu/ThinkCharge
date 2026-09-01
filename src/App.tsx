import { getCurrentWindow } from "@tauri-apps/api/window";
import { SettingsView } from "./components/SettingsView";
import { WidgetView } from "./components/WidgetView";

function App() {
  const label = getCurrentWindow().label;

  if (label === "settings") {
    return <SettingsView />;
  }

  return <WidgetView />;
}

export default App;
