import { Monitor, Moon, Sun } from "lucide-react";
import { useTheme, type ThemePreference } from "../hooks/useTheme";
import SegmentedControl from "./SegmentedControl";

export default function AppearanceSettings() {
  const { preference, setPreference, resolved } = useTheme();
  return (
    <section className="settings-section">
      <h2>Görünüm</h2>
      <p className="section-note">
        Windows temasını izler ya da açık/koyu görünüme sabitlenir. Tercih bu
        bilgisayarda saklanır.
      </p>
      <SegmentedControl<ThemePreference>
        label="Tema"
        value={preference}
        onChange={setPreference}
        options={[
          {
            value: "system",
            label: (
              <>
                <Monitor size={15} />
                Sistem
              </>
            ),
            description: `Şu an ${resolved === "dark" ? "koyu" : "açık"}`,
          },
          {
            value: "light",
            label: (
              <>
                <Sun size={15} />
                Açık
              </>
            ),
          },
          {
            value: "dark",
            label: (
              <>
                <Moon size={15} />
                Koyu
              </>
            ),
          },
        ]}
      />
    </section>
  );
}
