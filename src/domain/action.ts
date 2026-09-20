export const ToolAction = {
  Install: "install",
  Repair: "repair",
  Start: "start",
  Stop: "stop",
  Open: "open",
} as const;

export type ToolAction = (typeof ToolAction)[keyof typeof ToolAction];

export const GithubAction = {
  Save: "save",
  Forget: "forget",
  Import: "import",
} as const;

export type GithubAction = (typeof GithubAction)[keyof typeof GithubAction];

export const EnvironmentAction = {
  Start: "start",
  Stop: "stop",
} as const;

export type EnvironmentAction =
  (typeof EnvironmentAction)[keyof typeof EnvironmentAction];

export const PostgresSecretAction = {
  Credentials: "credentials",
  Password: "password",
} as const;

export type PostgresSecretAction =
  (typeof PostgresSecretAction)[keyof typeof PostgresSecretAction];

export const TunnelTokenAction = {
  Forget: "forget",
} as const;

export type TunnelTokenAction =
  (typeof TunnelTokenAction)[keyof typeof TunnelTokenAction];
