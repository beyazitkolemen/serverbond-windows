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
