import { call, desktop } from "../api";

export type CloudStatus = {
  paired: boolean;
  defaultUrl: string;
  connection:
    "disconnected" | "connecting" | "connected" | "retrying" | "revoked";
  socketEndpoint: string | null;
  url: string | null;
  name: string | null;
  account: string | null;
  lastContact: string | null;
  error: string | null;
};

export function normalizePairingCode(value: string): string {
  return value.replace(/[\s-]/g, "").toUpperCase();
}

export const cloudService = {
  status: (): Promise<CloudStatus> =>
    desktop
      ? call<CloudStatus>("cloud_status")
      : Promise.resolve({
          paired: false,
          defaultUrl: "https://serverbond.on-forge.com",
          connection: "disconnected",
          socketEndpoint: null,
          url: null,
          name: null,
          account: null,
          lastContact: null,
          error: null,
        }),
  pair: (code: string, url?: string) =>
    call<void>("cloud_pair", {
      code: normalizePairingCode(code),
      ...(url ? { url } : {}),
    }),
  disconnect: () => call<void>("cloud_disconnect"),
};
