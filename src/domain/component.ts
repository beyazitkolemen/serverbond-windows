/** Installed binary identity. Matches Rust `ComponentId` and `bin/<id>/`. */
export const ComponentId = {
  Php: "php",
  Mysql: "mysql",
  Caddy: "caddy",
  Composer: "composer",
  PhpMyAdmin: "phpmyadmin",
  Mailpit: "mailpit",
  Redis: "redis",
  Postgres: "postgres",
  Cloudflared: "cloudflared",
  Node: "node",
} as const;

export type ComponentId = (typeof ComponentId)[keyof typeof ComponentId];

export const CORE_COMPONENTS = [
  ComponentId.Php,
  ComponentId.Mysql,
  ComponentId.Caddy,
] as const;
