import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Github, AlertCircle, Check, X, Pencil, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useLauncherStore } from "../../../features/system/store/useLauncherStore";
import { useGameStore } from "../../../features/game/store/useGameStore";

export const UserProfile = () => {
    const { t } = useTranslation();
    const [playerName, setPlayerName] = useState("");
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState("");
    const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
    const [appVersion, setAppVersion] = useState("...");
    const { checking: launcherChecking, checkSelfUpdate } = useLauncherStore();
    const { buttonState, checkForUpdates } = useGameStore();

    const isCheckingUpdates = launcherChecking || buttonState === 'checking';

    useEffect(() => {
        // Load player name on mount
        invoke<string>("get_player_name_command").then((name) => {
            setPlayerName(name);
            setEditValue(name);
        });

        // Get app version
        getVersion().then(setAppVersion).catch(console.error);
    }, []);

    const handleSave = async () => {
        if (!editValue.trim()) return;

        setSaveStatus('saving');
        try {
            await invoke("set_player_name_command", { name: editValue });
            setPlayerName(editValue);
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
        setEditValue(playerName);
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
                        <span>{playerName}</span>
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
                        title="Verificar atualizações agora"
                    >
                        <RefreshCw className={`w-3 h-3 text-zinc-400 ${isCheckingUpdates ? 'animate-spin text-hyamber' : ''}`} />
                    </button>
                </div>
            </div>
            <div className="flex gap-5 ml-10">
                <span
                    className="text-white/60 hover:text-white cursor-pointer transition-colors"
                    onClick={() => invoke("open_url", { url: "https://github.com/BrunoF2P/hycore" })}
                    title={t("tooltips.github")}
                >
                    <Github className="w-5 h-5" />
                </span>
                <span
                    className="text-white/60 hover:text-white cursor-pointer transition-colors"
                    onClick={() => invoke("open_url", { url: "https://github.com/BrunoF2P/hycore/issues" })}
                    title={t("tooltips.report_issue")}
                >
                    <AlertCircle className="w-5 h-5" />
                </span>
            </div>


        </div>
    );
};
