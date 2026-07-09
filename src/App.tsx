import type { ReactElement } from "react";
import { useState } from "react";
import { Sidebar } from "./shared/layout/Sidebar";
import { Topbar } from "./shared/layout/Topbar";
import { DegradationBanner } from "./shared/layout/DegradationBanner";
import { KillSwitchProvider } from "./shared/components/KillSwitchProvider";
import { ToastProvider } from "./shared/components/ToastProvider";
import { Onboarding } from "./onboarding/Onboarding";
import { Dashboard } from "./dashboard/Dashboard";
import { Timeline } from "./timeline/Timeline";
import { Processes } from "./processes/Processes";
import { Destinations } from "./destinations/Destinations";
import { Inspector } from "./inspector/Inspector";
import { Search } from "./search/Search";
import { Alerts } from "./alerts/Alerts";
import { KillSwitch } from "./killswitch/KillSwitch";
import { Settings } from "./settings/Settings";
import { Privacy } from "./privacy/Privacy";
import { Logs } from "./logs/Logs";
import { History } from "./history/History";
import type { ScreenId } from "./shared/lib/types";

const ONBOARDING_DONE_KEY = "vitrail:onboarding-done";

function AppScreens(): ReactElement {
  const [screen, setScreen] = useState<ScreenId>(
    localStorage.getItem(ONBOARDING_DONE_KEY) ? "dashboard" : "onboarding",
  );
  const [selectedFlowId, setSelectedFlowId] = useState<string | null>(null);

  const navigate = (next: ScreenId): void => setScreen(next);
  const selectFlow = (id: string): void => {
    setSelectedFlowId(id);
    setScreen("inspector");
  };

  if (screen === "onboarding") {
    return (
      <div id="content">
        <Onboarding
          onDone={() => {
            localStorage.setItem(ONBOARDING_DONE_KEY, "1");
            navigate("dashboard");
          }}
        />
      </div>
    );
  }

  return (
    <div id="app">
      <Sidebar activeScreen={screen} onNavigate={navigate} />
      <div id="main">
        <Topbar screen={screen} />
        <DegradationBanner />
        <div id="content">
          <div className="screen">
            {screen === "dashboard" && <Dashboard onNavigate={navigate} />}
            {screen === "timeline" && <Timeline onNavigate={navigate} onSelectFlow={selectFlow} />}
            {screen === "processes" && <Processes onNavigate={navigate} />}
            {screen === "destinations" && <Destinations onNavigate={navigate} />}
            {screen === "inspector" && (
              <Inspector
                flowId={selectedFlowId}
                onBack={() => navigate("timeline")}
                onSelectProcess={() => undefined}
                onSelectDestination={() => undefined}
                onNavigate={navigate}
              />
            )}
            {screen === "search" && <Search onSelectFlow={selectFlow} />}
            {screen === "alerts" && <Alerts />}
            {screen === "killswitch" && <KillSwitch />}
            {screen === "settings" && <Settings onNavigate={navigate} />}
            {screen === "privacy" && <Privacy />}
            {screen === "logs" && <Logs />}
            {screen === "history" && <History />}
          </div>
        </div>
      </div>
    </div>
  );
}

export function App(): ReactElement {
  return (
    <ToastProvider>
      <KillSwitchProvider>
        <AppScreens />
      </KillSwitchProvider>
    </ToastProvider>
  );
}
