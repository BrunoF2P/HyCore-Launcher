import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Github, AlertCircle, Check, X, Pencil } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

export const UserProfile = () => {
    const { t } = useTranslation();
    const [playerName, setPlayerName] = useState("");
    const [isEditing, setIsEditing] = useState(false);
    const [editValue, setEditValue] = useState("");
    const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');

    useEffect(() => {
        // Load player name on mount
        invoke<string>("get_player_name_command").then((name) => {
            setPlayerName(name);
            setEditValue(name);
        });
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
                            title="Save"
                        >
                            <Check className="w-4 h-4 text-emerald-500" />
                        </button>
                        <button
                            onClick={handleCancel}
                            className="p-1 hover:bg-red-500/20 rounded transition-colors"
                            title="Cancel"
                        >
                            <X className="w-4 h-4 text-red-500" />
                        </button>
                    </div>
                ) : (
                    <div
                        className="text-sm font-bold text-white leading-tight cursor-pointer hover:text-hyamber transition-colors group relative flex items-center gap-2"
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
                            Click to edit
                        </span>
                    </div>
                )}
                <div className="text-[10px] text-zinc-500 font-medium uppercase tracking-wider">{t('footer.version')} 0.1.0</div>
            </div>
            <div className="flex gap-5 ml-10">
                <span
                    className="text-white/60 hover:text-white cursor-pointer transition-colors"
                    onClick={() => openUrl("https://github.com/BrunoF2P/hycore")}
                    title={t("tooltips.github")}
                >
                    <Github className="w-5 h-5" />
                </span>
                <span
                    className="text-white/60 hover:text-white cursor-pointer transition-colors"
                    onClick={() => openUrl("https://github.com/BrunoF2P/hycore/issues")}
                    title={t("tooltips.report_issue")}
                >
                    <AlertCircle className="w-5 h-5" />
                </span>
            </div>


        </div>
    );
};
