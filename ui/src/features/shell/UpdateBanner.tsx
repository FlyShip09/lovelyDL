// Checks once per launch for a newer lovelyDL when the setting allows it, and offers it in a
// quiet banner. A build without update signing simply finds nothing to offer.

import { useEffect, useState } from "react";

import { Button } from "../../components/Button";
import { Banner } from "../../components/chrome";
import { commands } from "../../ipc";
import { useApp } from "../../store/app";
import styles from "./shell.module.css";

export function UpdateBanner() {
  const allowed = useApp((st) => st.settings?.autoUpdateApp ?? false);
  const [version, setVersion] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  useEffect(() => {
    if (!allowed) return;
    let live = true;
    commands.appUpdateCheck().then(
      (found) => live && setVersion(found),
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [allowed]);
  return (
    <Banner open={version !== null} label={`lovelyDL ${version ?? ""} is available`}>
      <span className={styles.bannerText}>lovelyDL {version} is available.</span>
      <Button variant="plain" onClick={() => setVersion(null)}>
        Later
      </Button>
      <Button
        variant="primary"
        disabled={installing}
        onClick={() => {
          setInstalling(true);
          commands.appUpdateInstall().catch(() => setInstalling(false));
        }}
      >
        {installing ? "Installing…" : "Install and restart"}
      </Button>
    </Banner>
  );
}
