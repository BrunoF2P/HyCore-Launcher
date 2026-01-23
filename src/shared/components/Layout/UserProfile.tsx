import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Github, Bug, Check, X, Pencil, RefreshCw, Wifi, WifiOff } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useLauncherStore } from "../../../features/system/store/useLauncherStore";
import { useGameStore } from "../../../features/game/store/useGameStore";
import { useSettingsStore } from "../../../features/settings/store/useSettingsStore";

export const UserProfile = () => {
    const { t } = useTranslation();
    const { settings, updateSettings } = useSettingsStore();
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState("");
    const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
    const [appVersion, setAppVersion] = useState("...");
    const { checking: launcherChecking, checkSelfUpdate } = useLauncherStore();
    const { buttonState, checkForUpdates } = useGameStore();

    const isCheckingUpdates = launcherChecking || buttonState === 'checking';

    useEffect(() => {
        setEditValue(settings.player_name);
    }, [settings.player_name]);

    useEffect(() => {
        // Get app version
        getVersion().then(setAppVersion).catch(console.error);
    }, []);

    const handleSave = async () => {
        if (!editValue.trim()) return;

        setSaveStatus('saving');
        try {
            await updateSettings({ player_name: editValue.trim() });
            setSaveStatus('saved');
            setIsEditing(false);
            setTimeout(() => setSaveStatus('idle'), 2000);
        } catch (error) {
            console.error("Failed to save player name:", error);
            setSaveStatus('error');
            setTimeout(() => setSaveStatus('idle'), 2000);
        }
    };

    const handleCancel = () => {
        setEditValue(settings.player_name);
        setIsEditing(false);
        setSaveStatus('idle');
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') {
            handleSave();
        } else if (e.key === 'Escape') {
            handleCancel();
        }
    };
    const handleManualUpdateCheck = async () => {
        if (isCheckingUpdates) return;
        await Promise.all([
            checkSelfUpdate(),
            checkForUpdates()
        ]);
    };

    return (
        <div className="flex items-center gap-4">

            <div className="flex flex-col">
                {isEditing ? (
                    <div className="flex items-center gap-2">
                        <input
                            type="text"
                            value={editValue}
                            onChange={(e) => setEditValue(e.target.value)}
                            onKeyDown={handleKeyDown}
                            maxLength={32}
                            autoFocus
                            className="bg-zinc-800/50 border border-hyamber/50 rounded px-2 py-1 text-sm font-bold text-white outline-none focus:border-hyamber w-40"
                        />
                        <button
                            onClick={handleSave}
                            className="p-1 hover:bg-emerald-500/20 rounded transition-colors"
                            title={t('common.save')}
                        >
                            <Check className="w-4 h-4 text-emerald-500" />
                        </button>
                        <button
                            onClick={handleCancel}
                            className="p-1 hover:bg-red-500/20 rounded transition-colors"
                            title={t('common.cancel')}
                        >
                            <X className="w-4 h-4 text-red-500" />
                        </button>
                    </div>
                ) : (
                    <div
                        className="text-lg font-black text-white leading-tight cursor-pointer hover:text-hyamber transition-colors group relative flex items-center gap-2"
                        onClick={() => setIsEditing(true)}
                    >
                        <span>{settings.player_name}</span>
                        <Pencil className="w-3 h-3 text-zinc-500 group-hover:text-hyamber transition-colors" />
                        {saveStatus === 'saved' && (
                            <span className="text-emerald-500">
                                <Check className="w-3 h-3" />
                            </span>
                        )}
                        <span className="absolute bottom-full left-0 mb-1 px-2 py-1 bg-zinc-900 text-[10px] text-zinc-400 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none">
                            {t('management.click_to_edit')}
                        </span>
                    </div>
                )}
                <div className="flex items-center gap-2 mt-1">
                    <div className="text-[11px] text-zinc-500 font-bold uppercase tracking-[0.15em] opacity-50">{t('footer.version')} {appVersion}</div>
                    <button
                        onClick={handleManualUpdateCheck}
                        disabled={isCheckingUpdates}
                        className={`p-1 hover:bg-white/5 rounded-full transition-all cursor-pointer ${isCheckingUpdates ? 'opacity-100' : 'opacity-30 hover:opacity-100'}`}
                        title={t('tooltips.check_updates')}
                    >
                        <RefreshCw className={`w-3 h-3 text-zinc-400 ${isCheckingUpdates ? 'animate-spin text-hyamber' : ''}`} />
                    </button>
                </div>
            </div>
            <div className="flex items-center gap-1.5 ml-8 bg-zinc-900/40 p-1 rounded-xl border border-white/5 backdrop-blur-md">
                <button
                    className="w-10 h-10 flex items-center justify-center rounded-lg text-white/40 hover:text-white hover:bg-white/10 transition-all duration-300 active:scale-95 cursor-pointer"
                    onClick={() => invoke("open_url", { url: "https://github.com/BrunoF2P/hycore" })}
                    title={t("tooltips.github")}
                >
                    <Github className="w-5 h-5" />
                </button>
                <button
                    className="w-10 h-10 flex items-center justify-center rounded-lg text-white/40 hover:text-red-400 hover:bg-red-400/10 transition-all duration-300 active:scale-95 cursor-pointer"
                    onClick={() => invoke("open_url", { url: "https://github.com/BrunoF2P/hycore/issues" })}
                    title={t("tooltips.report_issue")}
                >
                    <Bug className="w-5 h-5" />
                </button>
                <div className="w-[1px] h-4 bg-white/10 mx-0.5"></div>
                <button
                    className={`w-10 h-10 flex items-center justify-center rounded-lg transition-all duration-500 active:scale-95 cursor-pointer ${settings.online_mode
                        ? 'text-sky-400 bg-sky-400/10 shadow-[0_0_15px_-3px_rgba(56,189,248,0.3)] hover:bg-sky-400/20 border border-sky-400/20'
                        : 'text-zinc-600 bg-white/5 hover:bg-white/10 hover:text-zinc-400 border border-transparent'
                        }`}
                    onClick={() => updateSettings({ online_mode: !settings.online_mode })}
                    title={settings.online_mode ? "Modo Online Ativo" : "Modo Online Inativo"}
                >
                    {settings.online_mode ? <Wifi className="w-5 h-5" /> : <WifiOff className="w-5 h-5" />}
                </button>
            </div>


        </div>
    );
};
