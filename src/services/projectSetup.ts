import { call } from "../api";
export interface SetupRequest {
  requestId: string;
  source: "github" | "git" | "laravel" | "existing";
  name: string;
  location: string;
  branch: string;
  phpVersion: string;
  installDependencies: boolean;
  composer: boolean;
  build: boolean;
}
export interface SetupReport {
  ready: boolean;
  phpVersion: string;
  sourceInspectionPending: boolean;
  checks: { id: string; ok: boolean; installable: boolean }[];
}
export const projectSetup = {
  check: (input: SetupRequest) =>
    call<SetupReport>("setup_preflight", { input }),
  run: (input: SetupRequest) => call("setup_project", { input }),
};
