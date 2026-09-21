export const Page = {
  Overview: "overview",
  Packages: "packages",
  Projects: "projects",
  Logs: "logs",
  Services: "services",
  Api: "api",
  Settings: "settings",
} as const;

export type Page = (typeof Page)[keyof typeof Page];
