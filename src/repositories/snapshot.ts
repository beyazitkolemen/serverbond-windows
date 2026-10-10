import { call, type ReadOptions } from "../api";
import type { Snapshot } from "../types";

export const snapshotRepository = {
  get: (options?: ReadOptions) =>
    call<Snapshot>("snapshot", undefined, options),
};
