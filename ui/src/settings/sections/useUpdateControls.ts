import { useCallback, useEffect, useState } from "react";
import * as api from "../../ipc/commands";
import type { UpdateStatusDto } from "../../types/bindings";

const IDLE_STATUS: UpdateStatusDto = {
  status: "idle",
  update: null,
  error: null,
  downloaded: null,
  total: null,
};

export interface UpdateControls {
  available: boolean;
  check: () => Promise<void>;
  checking: boolean;
  install: () => Promise<void>;
  installing: boolean;
  restart: () => Promise<void>;
  status: UpdateStatusDto;
}

export function useUpdateControls(): UpdateControls {
  const [status, setStatus] = useState<UpdateStatusDto>(IDLE_STATUS);

  useEffect(() => {
    const unlistenPromise = api.onUpdateStatus((payload) => setStatus(payload));
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const checking = status.status === "checking";
  const installing = status.status === "installing";
  const available = status.status === "available" && status.update?.available;
  const check = useCallback(async () => {
    setStatus({ ...IDLE_STATUS, status: "checking" });
    const next = await api.checkUpdate(true);
    setStatus(next);
  }, []);
  const install = useCallback(async () => {
    setStatus({ ...IDLE_STATUS, status: "installing" });
    const next = await api.downloadAndInstallUpdate();
    setStatus(next);
  }, []);
  const restart = useCallback(() => api.restartApp(), []);

  return {
    available: Boolean(available),
    check,
    checking,
    install,
    installing,
    restart,
    status,
  };
}
