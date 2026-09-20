import { call } from "../api";
import type { Snapshot } from "../types";

export const snapshotRepository = {
  get: () => call<Snapshot>("snapshot"),
};
