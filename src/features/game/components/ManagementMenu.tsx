import { useTranslation } from "react-i18next";
import { Puzzle, FolderOpen, Trash2, ShieldX } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface ManagementMenuProps {
    onOpenMods?: () => void;
}

export const ManagementMenu = ({ onOpenMods }: ManagementMenuProps) => {
    const { t } = useTranslation();

    const btnClass = "w-16 h-16 bg-black/45 backdrop-blur-md border border-white/10 rounded-lg flex flex-col items-center justify-center cursor-pointer transition-all duration-300 hover:bg-white/15 hover:border-white/30 hover:-translate-y-0.5 group shrink-0";
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
        if (confirm(t('management.confirm_clear'))) {
            try {
                await invoke("wipe_game_data");
                alert(t('management.clear_success'));
            } catch (error) {
                console.error("Failed to clear data:", error);
            }
        }
    };

    const handleUninstall = async () => {
        if (confirm(t('management.confirm_uninstall'))) {
            try {
                await invoke("uninstall_game");
                alert(t('management.uninstall_success'));
                window.location.reload();
            } catch (error) {
                console.error("Failed to uninstall:", error);
                alert(t('management.uninstall_failed') + ": " + error);
            }
        }
    };

    return (
        <aside className="w-fit flex flex-col gap-4 mt-0">
            <button className={btnClass} title={t('tooltips.control_mods')} onClick={onOpenMods}>
                <Puzzle className={`${iconClass} text-hyamber`} />
                <span className={labelClass}>{t('management.mods')}</span>
            </button>
            <button className={btnClass} title={t('tooltips.open_folder')} onClick={handleOpenFolder}>
                <FolderOpen className={iconClass} />
                <span className={labelClass}>{t('management.folder')}</span>
            </button>
            <button className={btnClass} title={t('tooltips.clear_data')} onClick={handleClearData}>
                <Trash2 className={iconClass} />
                <span className={labelClass}>{t('management.clear')}</span>
            </button>
            <button className={`${btnClass} hover:bg-red-500/20 hover:border-red-500/40`} title={t('tooltips.uninstall')} onClick={handleUninstall}>
                <ShieldX className={`${iconClass} text-red-400`} />
                <span className={labelClass}>{t('management.remove')}</span>
            </button>
        </aside >
    );
};
