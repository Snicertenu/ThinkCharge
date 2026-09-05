import { useEffect, useState } from "react";
import { Effect, getCurrentWindow } from "@tauri-apps/api/window";
import type { SystemTheme } from "../types";

export function useSystemTheme(enabled: boolean, applyWindowEffects = true) {
  const [theme, setTheme] = useState<SystemTheme>("dark");

  useEffect(() => {
    if (!enabled) {
      document.documentElement.dataset.theme = "dark";
      return;
    }

    const window = getCurrentWindow();

    const applyTheme = async () => {
      try {
        const current = await window.theme();
        const resolved: SystemTheme = current === "light" ? "light" : "dark";
        setTheme(resolved);
        document.documentElement.dataset.theme = resolved;

        if (import.meta.env.TAURI_ENV_PLATFORM === "windows" && applyWindowEffects) {
          const effectOptions =
            resolved === "dark" ? [Effect.Mica, Effect.Acrylic] : [Effect.Mica, Effect.Acrylic];
          for (const effect of effectOptions) {
            try {
              await window.setEffects({ effects: [effect] });
              break;
            } catch {
              // Fall through to the next Windows effect.
            }
          }
        }
      } catch {
        const prefersDark = globalThis.matchMedia("(prefers-color-scheme: dark)").matches;
        const resolved: SystemTheme = prefersDark ? "dark" : "light";
        setTheme(resolved);
        document.documentElement.dataset.theme = resolved;
      }
    };

    void applyTheme();

    const media = globalThis.matchMedia("(prefers-color-scheme: dark)");
    const onMediaChange = () => void applyTheme();
    media.addEventListener("change", onMediaChange);

    let unlisten: (() => void) | undefined;
    void window.onThemeChanged(({ payload: nextTheme }) => {
      const resolved: SystemTheme = nextTheme === "light" ? "light" : "dark";
      setTheme(resolved);
      document.documentElement.dataset.theme = resolved;
      void applyTheme();
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      media.removeEventListener("change", onMediaChange);
      unlisten?.();
    };
  }, [enabled, applyWindowEffects]);

  return theme;
}
