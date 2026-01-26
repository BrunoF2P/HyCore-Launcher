import { useTranslation } from "react-i18next";
import { useGameStore } from "../store/useGameStore";
import { useSettingsStore } from "../../settings/store/useSettingsStore";
import { Layers } from "lucide-react";
import { useState, useRef, useEffect } from "react";

export const VersionSelector = () => {
    const { t } = useTranslation();
    const { installedVersions, activeVersion, switchVersion, availableVersions, checkForUpdates, latestVersion, buttonState } = useGameStore();
    const { settings, updateSettings } = useSettingsStore();
    const [isOpen, setIsOpen] = useState(false);
    const menuRef = useRef<HTMLDivElement>(null);

    // Combine installed and available versions
    const allVersions = [...installedVersions.map(v => ({ ...v, installed: true }))];

    // Add all available versions discovered by backend
    availableVersions.forEach(version => {
        if (!allVersions.find(v => v.version === version)) {
            allVersions.push({
                version,
                channel: settings.channel,
                installed: false,
                size: null,
                last_modified: null,
                etag: null
            });
        }
    });

    // If active version isn't in either, add it as a placeholder
    if (activeVersion > 0 && !allVersions.find(v => v.version === activeVersion)) {
        allVersions.push({
            version: activeVersion,
            channel: settings.channel,
            installed: false,
            size: null,
            last_modified: null,
            etag: null
        });
    }

    // Sort versions (roughly by version number descending)
    allVersions.sort((a, b) => b.version - a.version);

    // Close menu when clicking outside
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
                setIsOpen(false);
            }
        };
        document.addEventListener("mousedown", handleClickOutside);
        return () => document.removeEventListener("mousedown", handleClickOutside);
    }, []);

    const handleSwitch = async (v: any) => {
        setIsOpen(false);
        try {
            // If switching to a version in a different channel, update settings
            if (v.channel !== settings.channel) {
                await updateSettings({ channel: v.channel });
            }
            await switchVersion(v.version);
            await checkForUpdates();
        } catch (err) {
            console.error('Failed to switch version:', err);
        }
    };

    // Always show the button if possible, but allow empty state in dropdown

    return (
        <div className="relative" ref={menuRef}>
            <button
                onClick={() => setIsOpen(!isOpen)}
                className={`min-w-fit px-3 h-16 bg-white/5 border border-white/10 rounded flex items-center gap-2 hover:bg-white/10 transition-all cursor-pointer group ${isOpen ? 'bg-white/15 border-white/30' : ''}`}
                title={t('management.versions')}
            >
                <Layers className={`w-5 h-5 transition-colors ${isOpen ? 'text-hyamber' : 'text-white/40 group-hover:text-white'}`} />
                <span className={`text-xs font-bold font-mono transition-colors ${isOpen ? 'text-white' : 'text-white/40 group-hover:text-white'}`}>
                    {activeVersion > 0 || latestVersion > 0 ? `v${activeVersion || latestVersion}` : '...'}
                </span>
            </button>

            {isOpen && (
                <div className="absolute bottom-full right-0 mb-4 w-60 bg-[#0c0f16]/95 backdrop-blur-xl border border-white/10 rounded-xl shadow-[0_20px_50px_rgba(0,0,0,0.5)] overflow-hidden animate-in slide-in-from-bottom-2 fade-in duration-200 z-50">
                    <div className="px-4 py-3 border-b border-white/5 bg-white/[0.02]">
                        <span className="text-[10px] font-bold uppercase tracking-widest text-white/30">{t('management.versions')}</span>
                    </div>
                    <div className="max-h-80 overflow-y-auto p-1.5 space-y-1 custom-scrollbar">
                        {allVersions.length === 0 ? (
                            <div className="px-3 py-6 text-center">
                                {buttonState === 'checking' ? (
                                    <span className="text-[10px] text-white/20 uppercase font-black tracking-widest animate-pulse">{t('footer.checking')}</span>
                                ) : (
                                    <span className="text-[10px] text-white/20 uppercase font-black tracking-widest">{t('mods.no_results')}</span>
                                )}
                            </div>
                        ) : (
                            allVersions.map((v) => (
                                <button
                                    key={`${v.channel}-${v.version}`}
                                    onClick={() => handleSwitch(v)}
                                    className={`w-full px-3 py-2.5 rounded-lg text-left text-sm flex items-center justify-between transition-all cursor-pointer group ${v.version === activeVersion
                                        ? 'bg-hyamber/10 text-hyamber'
                                        : 'hover:bg-white/5 text-white/60 hover:text-white'
                                        }`}
                                >
                                    <div className="flex flex-col">
                                        <div className="flex items-center gap-2">
                                            <span className="font-bold whitespace-nowrap">v{v.version}</span>
                                            {v.version === activeVersion && (
                                                <span className="text-[8px] bg-hyamber/20 text-hyamber px-1.5 py-0.5 rounded font-black uppercase tracking-tighter">Active</span>
                                            )}
                                            {!v.installed && (
                                                <span className="text-[8px] bg-sky-500/20 text-sky-400 px-1.5 py-0.5 rounded font-black uppercase tracking-tighter">Available</span>
                                            )}
                                        </div>
                                        <span className="text-[9px] opacity-40 uppercase font-medium mt-0.5">{v.channel}</span>
                                    </div>
                                    {v.version === activeVersion ? (
                                        <div className="w-1.5 h-1.5 rounded-full bg-hyamber shadow-[0_0_8px_rgba(255,179,0,0.5)]" />
                                    ) : (
                                        <div className="w-1.5 h-1.5 rounded-full bg-white/10 group-hover:bg-white/20 transition-colors" />
                                    )}
                                </button>
                            ))
                        )}
                    </div>
                </div>
            )}
        </div>
    );
};
