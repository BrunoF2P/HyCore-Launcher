import "./App.css";
import { useState, useEffect } from "react";

import { ActionBar } from "./shared/components/Layout/ActionBar";
import { ManagementMenu } from "./features/game/components/ManagementMenu";
import { NewsSection } from "./features/news/components/NewsSection";
import { UpdateOverlay } from "./features/game/components/UpdateOverlay";
import { useGameStore } from "./features/game/store/useGameStore";
import { ModsPage } from "./features/mods";
import { useLauncherStore } from "./features/system/store/useLauncherStore";

function App() {
  const [view, setView] = useState<'dashboard' | 'mods'>('dashboard');
  const checkForUpdates = useGameStore((state) => state.checkForUpdates);
  const checkSelfUpdate = useLauncherStore((state) => state.checkSelfUpdate);

  useEffect(() => {
    checkForUpdates();
    checkSelfUpdate();
  }, []);

  return (
    <div
      className="bg-cover bg-center w-screen h-screen flex flex-col relative antialiased font-sans bg-black text-white overflow-hidden"
      style={{ backgroundImage: "url('/content-upper-new-3840.jpg')" }}
    >
      <UpdateOverlay />
      <div className="absolute inset-0 bg-gradient-to-b from-black/20 via-transparent to-black/40 pointer-events-none"></div>

      {view === 'dashboard' ? (
        <>
          <main className="flex-1 min-h-0 flex flex-row px-10 pb-10 pt-32 z-10 overflow-visible items-center">
            <ManagementMenu onOpenMods={() => setView('mods')} />
            <div className="relative flex flex-col gap-5 ml-10">
              <img
                src="/logo.png"
                alt="Hytale Logo"
                className="h-24 drop-shadow-xl absolute bottom-full left-0 mb-6"
              />
              <NewsSection />
            </div>
          </main>
          <ActionBar />
        </>
      ) : (
        <div className="absolute inset-0 z-20 bg-[#121418]">
          <ModsPage onBack={() => setView('dashboard')} />
        </div>
      )}
    </div>
  );
}

export default App;
