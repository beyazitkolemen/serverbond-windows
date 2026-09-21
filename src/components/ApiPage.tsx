import type { ApiSettings as ApiValues, Run } from "../types";
import { apiService } from "../services";
import { useDraft } from "../hooks/useDraft";
import ApiSettings from "./ApiSettings";
import ApiReference from "./ApiReference";

export default function ApiPage({
  settings,
  busy,
  run,
}: {
  settings: ApiValues;
  busy: boolean;
  run: Run;
}) {
  const { values, setValues, dirty, reset } = useDraft(settings);
  return (
    <div className="api-page">
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void run("API ayarları kaydediliyor…", () => apiService.save(values));
        }}
      >
        <fieldset className="settings-fields" disabled={busy}>
          <ApiSettings
            values={values}
            onChange={(patch) =>
              setValues((current) => ({ ...current, ...patch }))
            }
            busy={busy}
            run={run}
            dirty={dirty}
          />
          <div className="settings-save">
            <span>
              {dirty
                ? "Kaydedilmemiş değişiklikler var"
                : "API ayarları güncel"}
            </span>
            <button
              type="button"
              className="button secondary"
              disabled={!dirty || busy}
              onClick={reset}
            >
              Vazgeç
            </button>
            <button
              type="submit"
              className="button primary"
              disabled={!dirty || busy}
            >
              API ayarlarını kaydet
            </button>
          </div>
        </fieldset>
      </form>
      <ApiReference run={run} />
    </div>
  );
}
