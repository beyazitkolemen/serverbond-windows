import { call, desktop } from "../api";

export type CloudStatus = {
  paired: boolean;
  url: string | null;
  name: string | null;
  account: string | null;
  lastContact: string | null;
  error: string | null;
};

export const cloudService = {
  status: (): Promise<CloudStatus> =>
    desktop
      ? call<CloudStatus>("cloud_status")
      : Promise.resolve({
          paired: false,
          url: null,
          name: null,
          account: null,
          lastContact: null,
          error: null,
        }),
  pair: (url: string, code: string) => call<void>("cloud_pair", { url, code }),
  disconnect: () => call<void>("cloud_disconnect"),
};
