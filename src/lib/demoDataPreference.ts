import { useEffect, useState } from "react";
import { getAppSetting, setAppSetting } from "./backend";

const SETTING_KEY = "show_demo_data";

/**
 * Shared "Show Demo Data" preference (Sprint 008.1). Persisted via the
 * existing app_meta key/value pattern — no new table. Defaults to shown
 * (true) so nothing disappears unexpectedly on first run; each page that
 * lists operations filters demo rows out client-side when this is false.
 * Demo rows are never deleted or altered — this only affects what's
 * rendered.
 */
export function useShowDemoData(): [boolean, (value: boolean) => void] {
  const [show, setShow] = useState(true);

  useEffect(() => {
    let live = true;
    getAppSetting(SETTING_KEY).then((v) => {
      if (live && v !== null) setShow(v === "1");
    });
    return () => {
      live = false;
    };
  }, []);

  function update(value: boolean) {
    setShow(value);
    setAppSetting(SETTING_KEY, value ? "1" : "0");
  }

  return [show, update];
}
