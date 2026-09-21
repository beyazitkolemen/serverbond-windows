import type { ApiRoute } from "../types";

type Operation = {
  requestBody?: { content?: { "application/json"?: { schema?: unknown } } };
};

export default function ApiRequestSchema({
  document,
  route,
}: {
  document: Record<string, unknown> | null;
  route: ApiRoute;
}) {
  const paths = document?.paths as
    Record<string, Record<string, Operation>> | undefined;
  const path = route.path.split("?")[0].replace(/^\/api\/v1/, "") || "/";
  const schema =
    paths?.[path]?.[route.method.toLowerCase()]?.requestBody?.content?.[
      "application/json"
    ]?.schema;
  if (!schema) return null;
  return (
    <details className="api-body-schema">
      <summary>JSON gövdesi şeması</summary>
      <pre>{JSON.stringify(schema, null, 2)}</pre>
    </details>
  );
}
