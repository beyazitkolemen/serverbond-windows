import { call } from "../api";
import type {
  GithubAuthFlow,
  GithubAuthPoll,
  GithubRepoPage,
  GithubBranchPage,
} from "../types";
import {
  GithubAction,
  GithubCommand,
  PostgresSecretAction,
  ToolAction,
  ToolCommand,
  TunnelTokenAction,
  type ToolAction as ToolActionName,
  type ToolCommand as ToolCommandName,
} from "../domain";

export function runTool(command: ToolCommandName, action: ToolActionName) {
  return call(command, { action });
}

function lifecycle(command: ToolCommandName) {
  return {
    install: () => runTool(command, ToolAction.Install),
    repair: () => runTool(command, ToolAction.Repair),
    start: () => runTool(command, ToolAction.Start),
    stop: () => runTool(command, ToolAction.Stop),
    open: () => runTool(command, ToolAction.Open),
    act: (action: ToolActionName) => runTool(command, action),
  };
}

export const mailService = lifecycle(ToolCommand.Mail);
export const redisService = lifecycle(ToolCommand.Redis);
export const tunnelService = {
  ...lifecycle(ToolCommand.Tunnel),
  forget: () => call(ToolCommand.Tunnel, { action: TunnelTokenAction.Forget }),
  saveToken: (token: string, start: boolean) =>
    call("save_tunnel_token", { token, start }),
  saveAutoStart: (autoStart: boolean) =>
    call("save_tunnel_auto_start", { autoStart }),
};
export const nodeService = lifecycle(ToolCommand.Node);
export const postgresService = {
  ...lifecycle(ToolCommand.Postgres),
  credentials: () =>
    call<string>(ToolCommand.Postgres, {
      action: PostgresSecretAction.Credentials,
    }),
  changePassword: (password: string) =>
    call(ToolCommand.Postgres, {
      action: PostgresSecretAction.Password,
      password,
    }),
};
export const githubService = {
  saveClientId: (clientId: string) =>
    call(GithubCommand.AuthSettings, { clientId }),
  authStart: () => call<GithubAuthFlow>(GithubCommand.AuthStart),
  authPoll: (flowId: string) =>
    call<GithubAuthPoll>(GithubCommand.AuthPoll, { flowId }),
  authCancel: (flowId: string) => call(GithubCommand.AuthCancel, { flowId }),
  authOpen: () => call(GithubCommand.AuthOpen),
  repositories: (page = 1) =>
    call<GithubRepoPage>(GithubCommand.Repositories, { page }),
  branches: (repository: string, page = 1) =>
    call<GithubBranchPage>(GithubCommand.Branches, { repository, page }),
  save: (token: string) =>
    call(ToolCommand.Github, { action: GithubAction.Save, token }),
  forget: () => call(ToolCommand.Github, { action: GithubAction.Forget }),
  import: (input: { repository: string; name: string; branch: string }) =>
    call(ToolCommand.Github, { action: GithubAction.Import, ...input }),
};
