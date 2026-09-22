import type { ApiSettings as ApiValues, Run } from "../types";
import { apiService } from "../services";
import { useDraft } from "../hooks/useDraft";
import ApiSettings from "./ApiSettings";
import ApiReference from "./ApiReference";
import { useId, useState } from "react";
import SectionTabs from "./SectionTabs";
import SaveBar from "./SaveBar";

export default function ApiPage({
  settings,
  busy,
  run,
}: {
  settings: ApiValues;
  busy: boolean;
  run: Run;
}) {
  const { values, setValues, dirty, baseline, conflicted, reset } =
    useDraft(settings);
  const [section, setSection] = useState<"connection" | "reference">(
    "connection",
  );
  const tabsId = useId();
  return (
    <div className="api-page">
      <SectionTabs
        id={tabsId}
        label="API bölümleri"
        value={section}
        onChange={setSection}
        items={[
          {
            id: "connection",
            label: "Bağlantı ve erişim",
            indicator: dirty ? (
              <span
                className="draft-dot"
                aria-label="Kaydedilmemiş değişiklikler"
              />
            ) : null,
          },
          { id: "reference", label: "Uç nokta rehberi" },
        ]}
      />
      <div
        id={`${tabsId}-panel`}
        role="tabpanel"
        aria-labelledby={`${tabsId}-${section}`}
        tabIndex={0}
      >
        <div hidden={section !== "connection"}>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (busy || !dirty || conflicted) return;
              void run("API ayarları kaydediliyor…", () =>
                apiService.save(values, baseline),
              );
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
              <SaveBar
                dirty={dirty}
                busy={busy}
                canSave={dirty && !busy && !conflicted}
                blocked={
                  conflicted
                    ? "API ayarları başka bir işlemde değişti. Vazgeç ile güncel ayarları alın."
                    : undefined
                }
                onReset={reset}
                label="API ayarlarını kaydet"
              />
            </fieldset>
          </form>
        </div>
        <div hidden={section !== "reference"}>
          <ApiReference run={run} />
        </div>
      </div>
    </div>
  );
}
