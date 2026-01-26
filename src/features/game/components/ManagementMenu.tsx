import { useTranslation } from "react-i18next";
import { Puzzle, FolderOpen, Trash2, ShieldX, Settings } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../../settings/store/useSettingsStore";
import { useGameStore } from "../store/useGameStore";

interface ManagementMenuProps {
    onOpenMods?: () => void;
    onOpenSettings?: () => void;
}

export const ManagementMenu = ({ onOpenMods, onOpenSettings }: ManagementMenuProps) => {
    const { t } = useTranslation();
    const settings = useSettingsStore(state => state.settings);
    const installedVersions = useGameStore(state => state.installedVersions);
    const activeVersion = useGameStore(state => state.activeVersion);

    const activeInfo = installedVersions.find(v => v.version === activeVersion);
    const isPreRelease = activeInfo?.channel === 'pre-release' || settings.channel === 'pre-release';

    const btnClass = "w-16 h-16 bg-black/45 backdrop-blur-md border border-white/10 rounded-lg flex flex-col items-center justify-center cursor-pointer transition-all duration-300 hover:bg-white/15 hover:border-white/30 hover:-translate-y-0.5 active:translate-y-0 active:scale-95 group shrink-0 relative focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hyamber/50 focus-visible:border-hyamber/50";
    const iconClass = "w-6 h-6 mb-1 opacity-90 transition-transform group-hover:scale-110";
    const labelClass = "text-[9px] uppercase font-bold tracking-tight opacity-70 group-hover:opacity-100";

    const handleOpenFolder = async () => {
        try {
            await invoke("open_game_folder");
        } catch (error) {
            console.error("Failed to open folder:", error);
        }
    };

    const handleClearData = async () => {
        const confirmed = await ask(t('management.confirm_clear'), {
            title: t('management.clear'),
            kind: 'warning'
        });

        if (confirmed) {
            try {
                await invoke("wipe_game_data");
                await message(t('management.clear_success'), { title: 'HyCore' });
            } catch (error) {
                console.error("Failed to clear data:", error);
            }
        }
    };

    const handleUninstall = async () => {
        const confirmed = await ask(t('management.confirm_uninstall'), {
            title: t('management.remove'),
            kind: 'warning'
        });

        if (confirmed) {
            try {
                await invoke("uninstall_game");
                await message(t('management.uninstall_success'), { title: 'HyCore' });
                window.location.reload();
            } catch (error) {
                console.error("Failed to uninstall:", error);
                await message(`${t('management.uninstall_failed')}: ${error}`, { title: 'Error', kind: 'error' });
            }
        }
    };

    return (
        <aside className="w-fit flex flex-col gap-4 mt-0" role="navigation" aria-label={t('management.menu_label') || "Management Menu"}>
            <button
                className={btnClass}
                title={t('tooltips.settings')}
                onClick={onOpenSettings}
                aria-label={t('management.settings')}
            >
                <Settings className={iconClass} aria-hidden="true" />
                <span className={labelClass}>{t('management.settings')}</span>
            </button>
            <button
                className={btnClass}
                title={t('tooltips.control_mods')}
                onClick={onOpenMods}
                aria-label={t('management.mods')}
            >
                {isPreRelease && (
                    <div className="absolute -top-1 -right-1 bg-amber-500 text-black text-[8px] font-black px-1.5 py-0.5 rounded-full border border-black shadow-lg z-10" aria-label={t('management.dev_badge')}>
                        {t('management.dev_badge')}
                    </div>
                )}
                <Puzzle className={`${iconClass} text-hyamber`} aria-hidden="true" />
                <span className={labelClass}>{t('management.mods')}</span>
            </button>
            <button
                className={btnClass}
                title={t('tooltips.open_folder')}
                onClick={handleOpenFolder}
                aria-label={t('management.folder')}
            >
                <FolderOpen className={iconClass} aria-hidden="true" />
                <span className={labelClass}>{t('management.folder')}</span>
            </button>
            <button
                className={btnClass}
                title={t('tooltips.clear_data')}
                onClick={handleClearData}
                aria-label={t('management.clear')}
            >
                <Trash2 className={iconClass} aria-hidden="true" />
                <span className={labelClass}>{t('management.clear')}</span>
            </button>
            <button
                className={`${btnClass} hover:bg-red-500/20 hover:border-red-500/40`}
                title={t('tooltips.uninstall')}
                onClick={handleUninstall}
                aria-label={t('management.remove')}
            >
                <ShieldX className={`${iconClass} text-red-400`} aria-hidden="true" />
                <span className={labelClass}>{t('management.remove')}</span>
            </button>
        </aside >
    );
};
