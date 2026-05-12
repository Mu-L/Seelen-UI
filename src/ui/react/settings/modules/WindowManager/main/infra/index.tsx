import { ConfigProvider, Switch } from "antd";
import { useTranslation } from "react-i18next";

import { BorderSettings } from "../../border/infra.tsx";

import { getWmConfig, setWmEnabled } from "../../application.ts";

import { SettingsGroup, SettingsOption } from "../../../../components/SettingsBox/index.tsx";
import { WmAnimationsSettings } from "./Animations.tsx";
import { GlobalPaddings } from "./GlobalPaddings.tsx";
import { OthersConfigs } from "./Others.tsx";
import { LayoutSelector } from "./LayoutSelector.tsx";

export function WindowManagerSettings() {
  const wmSettings = getWmConfig();

  const { t } = useTranslation();

  const onToggleEnable = (value: boolean) => {
    setWmEnabled(value);
  };

  return (
    <>
      <SettingsGroup>
        <SettingsOption>
          <div>
            <b>{t("wm.enable")}</b>
          </div>
          <Switch checked={wmSettings.enabled} onChange={onToggleEnable} />
        </SettingsOption>
      </SettingsGroup>

      <ConfigProvider componentDisabled={!wmSettings.enabled}>
        <LayoutSelector />
        <GlobalPaddings />
        <BorderSettings />
        <WmAnimationsSettings />
        <OthersConfigs />
      </ConfigProvider>
    </>
  );
}
