import { call } from "../api";
import {
  EnvironmentAction,
  type ComponentId as ComponentName,
  type EnvironmentAction as EnvironmentActionName,
} from "../domain";
import type {
  ApiStatus,
  ApiSettings,
  ApiDocumentation,
  Settings,
} from "../types";

export const environmentService = {
  start: (id = "all") =>
    call("service", { id, action: EnvironmentAction.Start }),
  stop: (id = "all") => call("service", { id, action: EnvironmentAction.Stop }),
  toggle: (running: boolean, id = "all") =>
    call("service", {
      id,
      action: running ? EnvironmentAction.Stop : EnvironmentAction.Start,
    }),
  act: (id: string, action: EnvironmentActionName) =>
    call("service", { id, action }),
};

export const packagesService = {
  install: (id: ComponentName | "all") => call("install", { id }),
  repair: (id: ComponentName) => call("repair", { id }),
  openPhpMyAdmin: () => call("open_phpmyadmin"),
};

export const settingsService = {
  save: (settings: Settings, expected: Settings) =>
    call("save_settings", { settings, expected }),
};

export const apiService = {
  status: () => call<ApiStatus>("api_status"),
  save: (settings: ApiSettings, expected: ApiSettings) =>
    call("api_save", { settings, expected }),
  documentation: () => call<ApiDocumentation>("api_documentation"),
  createToken: () => call<string>("api_token", { action: "create" }),
  forgetToken: () => call<string>("api_token", { action: "forget" }),
};
