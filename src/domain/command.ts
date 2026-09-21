/**
 * IPC command names. These are not always the process / `bin/` identity.
 * Example: command `mail` drives process `mailpit`.
 */
export const ToolCommand = {
  Mail: "mail",
  Redis: "redis",
  Postgres: "postgres",
  Tunnel: "tunnel",
  Node: "node",
  Github: "github",
} as const;

export type ToolCommand = (typeof ToolCommand)[keyof typeof ToolCommand];

export const GithubCommand = {
  AuthSettings: "github_auth_settings",
  AuthStart: "github_auth_start",
  AuthPoll: "github_auth_poll",
  AuthCancel: "github_auth_cancel",
  AuthOpen: "github_auth_open",
  Repositories: "github_repositories",
  Branches: "github_branches",
} as const;

/** Hizmetler page catalog ids. `pma` is the UI tile, not `phpmyadmin`. */
export const WorkspaceService = {
  PhpMyAdmin: "pma",
  Mail: "mail",
  Postgres: "postgres",
  Redis: "redis",
  Github: "github",
  Tunnel: "tunnel",
} as const;

export type WorkspaceService =
  (typeof WorkspaceService)[keyof typeof WorkspaceService];
