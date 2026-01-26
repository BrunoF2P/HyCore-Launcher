import "./App.css";
import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";

import { ActionBar } from "./shared/components/Layout/ActionBar";
import { ManagementMenu } from "./features/game/components/ManagementMenu";
import { NewsSection } from "./features/news/components/NewsSection";
import { UpdateOverlay } from "./features/game/components/UpdateOverlay";
import { useGameStore } from "./features/game/store/useGameStore";
import { useLauncherStore } from "./features/system/store/useLauncherStore";
import { useSettingsStore } from "./features/settings/store/useSettingsStore";
import { Suspense, lazy } from "react";

const ModsPage = lazy(() => import("./features/mods/components/ModsLayout"));
const SettingsOverlay = lazy(() => import("./features/settings/components/SettingsOverlay").then(m => ({ default: m.SettingsOverlay })));



function App() {
  const { t } = useTranslation();
  const [view, setView] = useState<'dashboard' | 'mods'>('dashboard');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const fatalError = useLauncherStore(state => state.fatalError);
  const checkForUpdates = useGameStore(state => state.checkForUpdates);
  const checkSelfUpdate = useLauncherStore(state => state.checkSelfUpdate);
  const loadSettings = useSettingsStore(state => state.loadSettings);
  const initLauncher = useLauncherStore(state => state.init);

  useEffect(() => {
    initLauncher();
    loadSettings();
    checkForUpdates();
    checkSelfUpdate();
  }, [initLauncher, loadSettings, checkForUpdates, checkSelfUpdate]);

  if (fatalError) {
    return (
      <div className="w-screen h-screen bg-[#0c0f16] flex flex-col items-center justify-center p-10 text-center relative z-[1000]">
        <div className="max-w-md bg-red-950/20 p-8 rounded-2xl border border-red-500/30 backdrop-blur-xl shadow-2xl shadow-red-900/10" role="alert" aria-labelledby="fatal-error-title">
          <div className="text-red-500 mb-6 flex justify-center">
            <svg className="w-16 h-16 drop-shadow-[0_0_10px_rgba(239,68,68,0.3)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <h1 id="fatal-error-title" className="text-2xl font-black mb-4 tracking-tight uppercase">{t('error.fatal_title')}</h1>
          <p className="text-white/70 mb-8 leading-relaxed text-sm font-medium">
            {fatalError}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="px-8 py-2.5 bg-red-600 hover:bg-red-500 text-white rounded-lg transition-all font-bold uppercase tracking-widest text-xs active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500/50"
          >
            {t('error.try_again')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      className="bg-cover bg-center w-screen h-screen flex flex-col relative antialiased font-sans bg-black text-white overflow-hidden"
      style={{ backgroundImage: "url('/content-upper-new-3840.jpg')" }}
    >
      <UpdateOverlay />
      <div className="absolute inset-0 bg-gradient-to-b from-black/20 via-transparent to-black/40 pointer-events-none"></div>

      {view === 'dashboard' ? (
        <>
          <main className="flex-1 min-h-0 flex flex-row px-20 pb-14 pt-36 z-10 overflow-visible items-center">
            <ManagementMenu
              onOpenMods={() => setView('mods')}
              onOpenSettings={() => setSettingsOpen(true)}
            />
            <div className="relative flex flex-col gap-6 ml-14 flex-1">
              <img
                src="/logo.png"
                alt={t('common.logo_alt')}
                className="h-28 drop-shadow-[0_20px_50px_rgba(0,0,0,0.5)] absolute bottom-full left-0 mb-8 pointer-events-none"
              />
              <NewsSection />
            </div>
          </main>
          <ActionBar />
          {settingsOpen && (
            <Suspense fallback={null}>
              <SettingsOverlay onClose={() => setSettingsOpen(false)} />
            </Suspense>
          )}
        </>
      ) : (
        <div className="absolute inset-0 z-20 bg-[#0c0f16]">
          <Suspense fallback={<div className="flex items-center justify-center h-full text-white/20">{t('mods.loading')}</div>}>
            <ModsPage onBack={() => setView('dashboard')} />
          </Suspense>
        </div>
      )}
    </div>
  );
}

export default App;
