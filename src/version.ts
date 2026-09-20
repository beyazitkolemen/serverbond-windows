import pkg from "../package.json" with { type: "json" };

export const APP_VERSION: string = pkg.version;

export function phpSupportsLaravel12(version: string): boolean {
  const [major = 0, minor = 0] = version.split(".").map((part) => Number(part));
  return (
    Number.isInteger(major) &&
    Number.isInteger(minor) &&
    (major > 8 || (major === 8 && minor >= 2))
  );
}

export function projectAddress(pattern: string, name: string): string {
  return pattern.replaceAll("{name}", name || "ornek-proje");
}

export function projectUrl(
  host: string,
  webPort: number,
  https: boolean,
  httpsPort: number,
): string {
  return https ? `https://${host}:${httpsPort}` : `http://${host}:${webPort}`;
}

export function databaseName(name: string): string {
  return name.replaceAll("-", "_");
}
