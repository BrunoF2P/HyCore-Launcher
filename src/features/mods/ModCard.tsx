import { CurseForgeMod, InstalledMod } from './types';
import { Download, Trash2, Check, X, Calendar, ArrowUpCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ModCardProps {
    mod: CurseForgeMod | InstalledMod;
    isInstalled: boolean;
    onInstall?: (mod: CurseForgeMod) => void;
    onRemove?: (mod: InstalledMod) => void;
    onToggle?: (mod: InstalledMod, enabled: boolean) => void;
    onUpdate?: (mod: InstalledMod) => void;
    isLoading?: boolean;
    onClick?: () => void;
}

import { memo } from 'react';

const ModCard = memo(function ModCard({ mod, isInstalled, onInstall, onRemove, onToggle, onUpdate, isLoading, onClick }: ModCardProps) {
    const { t } = useTranslation();
    // Helper type guards
    const isCF = (m: any): m is CurseForgeMod => 'latestFiles' in m;
    const isInst = (m: any): m is InstalledMod => 'filePath' in m;

    const thumbnail = isCF(mod) ? mod.logo?.thumbnailUrl : mod.iconUrl;
    const description = isCF(mod) ? mod.summary : mod.description;
    const author = isCF(mod) ? (mod.authors[0]?.name || 'Unknown') : mod.author;
    const downloads = isCF(mod) ? mod.downloadCount : mod.downloads;
    const hasUpdate = isInst(mod) && mod.latestFileId !== null && mod.latestFileId !== undefined;

    // Format downloads
    const formatDownloads = (num?: number) => {
        if (!num) return '0';
        if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
        if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
        return num.toString();
    };

    return (
        <div
            onClick={onClick}
            className={`bg-white/5 backdrop-blur-sm border rounded-xl p-4 hover:border-white/20 transition-all flex flex-col h-full group relative overflow-hidden cursor-pointer ${hasUpdate ? 'border-amber-500/40 shadow-[0_0_15px_rgba(245,158,11,0.1)]' : 'border-white/10'
                }`}
        >
            {hasUpdate && (
                <div className="absolute top-2 right-2 z-20 flex items-center gap-1 bg-amber-500 text-black text-[9px] font-black uppercase tracking-tighter px-2 py-0.5 rounded-full shadow-lg">
                    <ArrowUpCircle size={10} /> Update
                </div>
            )}

            {/* Glossy overlay effect */}
            <div className="absolute inset-0 bg-gradient-to-br from-white/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none" />

            <div className="flex gap-4 mb-3 relative z-10">
                <div className="w-16 h-16 rounded-lg bg-black/40 flex-shrink-0 overflow-hidden border border-white/5">
                    {thumbnail ? (
                        <img src={thumbnail} alt={mod.name} className="w-full h-full object-cover" />
                    ) : (
                        <div className="w-full h-full flex items-center justify-center text-white/20">
                            <span className="text-xs">{t('mods.no_icon')}</span>
                        </div>
                    )}
                </div>
                <div className="flex-1 min-w-0">
                    <h3 className="text-white font-bold text-lg truncate leading-tight mb-1" title={mod.name}>
                        {mod.name}
                    </h3>
                    <p className="text-white/50 text-xs truncate mb-1">{t('mods.by')} {author}</p>
                    <div className="flex items-center gap-2 text-[10px] text-white/40">
                        <span className="flex items-center gap-1"><Download size={8} /> {formatDownloads(downloads)}</span>
                        {isCF(mod) && <span className="flex items-center gap-1"><Calendar size={8} /> {new Date(mod.dateModified).toLocaleDateString()}</span>}
                        {isInst(mod) && <span className="bg-white/5 px-1.5 rounded-md">v{mod.version}</span>}
                    </div>
                </div>
            </div>

            <p className="text-white/70 text-sm line-clamp-2 mb-4 flex-1 relative z-10 min-h-[40px]">
                {description}
            </p>

            <div className="mt-auto flex gap-2 relative z-10">
                {isInstalled && isInst(mod) ? (
                    <>
                        {hasUpdate ? (
                            <button
                                onClick={(e) => { e.stopPropagation(); onUpdate?.(mod); }}
                                disabled={isLoading}
                                className="flex-[2] flex items-center justify-center gap-2 py-2 bg-amber-500 text-black rounded-lg text-sm font-black uppercase tracking-wider hover:bg-amber-400 transition-all shadow-lg shadow-amber-500/20"
                            >
                                <ArrowUpCircle size={14} /> Update to {mod.latestVersion}
                            </button>
                        ) : (
                            <button
                                onClick={(e) => { e.stopPropagation(); onToggle?.(mod, !mod.enabled); }}
                                className={`flex-1 flex items-center justify-center gap-2 py-2 rounded-lg text-sm font-medium transition-colors ${mod.enabled
                                    ? 'bg-sky-500/20 text-sky-400 hover:bg-sky-500/30'
                                    : 'bg-white/5 text-white/50 hover:bg-white/10'
                                    }`}
                            >
                                {mod.enabled ? <><Check size={12} /> {t('mods.enabled')}</> : <><X size={12} /> {t('mods.disabled')}</>}
                            </button>
                        )}
                        <button
                            onClick={(e) => { e.stopPropagation(); onRemove?.(mod); }}
                            disabled={isLoading}
                            className={`px-3 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-500 rounded-lg transition-colors disabled:opacity-50 ${hasUpdate ? 'flex-1' : ''}`}
                            title={t('tooltips.uninstall')}
                        >
                            <Trash2 size={14} />
                        </button>
                    </>
                ) : (
                    <button
                        onClick={() => isCF(mod) && onInstall?.(mod)}
                        disabled={isLoading || isInstalled} // Could happen if ID check matches
                        className={`w-full py-2 rounded-lg text-sm font-bold flex items-center justify-center gap-2 transition-all ${isInstalled
                            ? 'bg-white/5 text-white/30 cursor-not-allowed'
                            : 'bg-gradient-to-r from-sky-500 to-blue-500 text-white hover:brightness-110'
                            }`}
                    >
                        {isLoading ? (
                            <span className="animate-pulse">{t('mods.installing')}</span>
                        ) : isInstalled ? (
                            <>{t('mods.installed_btn')}</>
                        ) : (
                            <><Download size={12} /> {t('mods.install')}</>
                        )}
                    </button>
                )}
            </div>
        </div>
    );
});

export default ModCard;


