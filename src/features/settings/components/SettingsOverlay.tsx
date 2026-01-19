import { useTranslation } from "react-i18next";
import { X, Cpu, Save, Settings, Bot } from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../store/useSettingsStore";

interface SettingsOverlayProps {
    onClose: () => void;
}

export const SettingsOverlay = ({ onClose }: SettingsOverlayProps) => {
    const { t } = useTranslation();
    const { settings, loadSettings, updateSettings, loading } = useSettingsStore();
    const [localSettings, setLocalSettings] = useState(settings);
    const [systemRamGb, setSystemRamGb] = useState<number>(32); // Default fallback

    useEffect(() => {
        loadSettings();
        // Fetch system RAM
        invoke<number>('get_system_ram_gb').then((ram) => {
            setSystemRamGb(ram);
        }).catch((err) => {
            console.error('Failed to get system RAM:', err);
        });
    }, []);

    useEffect(() => {
        setLocalSettings(settings);
    }, [settings]);

    const handleSave = async () => {
        await updateSettings(localSettings);
        onClose();
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-10">
            <div className="absolute inset-0 bg-black/60 backdrop-blur-xl animate-in fade-in duration-300" onClick={onClose}></div>

            <div className="relative w-full max-w-2xl bg-[#0c0f16] border border-white/5 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[80vh] animate-in zoom-in-95 fade-in duration-300">
                {/* Header */}
                <div className="flex items-center justify-between px-8 py-6 border-b border-white/5 bg-white/[0.02]">
                    <div className="flex items-center gap-3">
                        <div className="p-2 bg-sky-500/10 rounded-lg">
                            <Settings className="w-5 h-5 text-sky-400" />
                        </div>
                        <div>
                            <h2 className="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-sky-400 to-blue-500">{t('settings.title')}</h2>
                            <p className="text-xs text-white/40 uppercase tracking-widest font-medium mt-0.5">{t('settings.subtitle')}</p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-white/5 rounded-full transition-colors group"
                    >
                        <X className="w-6 h-6 text-white/40 group-hover:text-white" />
                    </button>
                </div>

                {/* Content */}
                <div className="flex-1 overflow-y-auto p-8 space-y-10 custom-scrollbar">

                    {/* Java Performance section */}
                    <section className="space-y-6">
                        <div className="flex items-center gap-2 mb-2">
                            <Cpu className="w-4 h-4 text-sky-400/60" />
                            <h3 className="text-sm font-bold uppercase tracking-wider text-white/50">{t('settings.java_performance')}</h3>
                        </div>

                        <div className="grid grid-cols-2 gap-6">
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-white/70 flex justify-between">
                                    <span>{t('settings.ram_min')}</span>
                                    <span className="text-sky-400">{localSettings.ram_min_gb} GB</span>
                                </label>
                                <input
                                    type="range"
                                    min="1"
                                    max={Math.max(4, Math.floor(systemRamGb / 2))}
                                    step="1"
                                    value={localSettings.ram_min_gb}
                                    onChange={(e) => setLocalSettings({ ...localSettings, ram_min_gb: parseInt(e.target.value) })}
                                    className="w-full accent-sky-400 h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer"
                                />
                            </div>
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-white/70 flex justify-between">
                                    <span>{t('settings.ram_max')}</span>
                                    <span className="text-sky-400">{localSettings.ram_max_gb} GB</span>
                                </label>
                                <input
                                    type="range"
                                    min="2"
                                    max={systemRamGb}
                                    step="1"
                                    value={localSettings.ram_max_gb}
                                    onChange={(e) => setLocalSettings({ ...localSettings, ram_max_gb: parseInt(e.target.value) })}
                                    className="w-full accent-sky-400 h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer"
                                />
                            </div>
                        </div>

                        <div className="space-y-2">
                            <label className="text-sm font-medium text-white/70">{t('settings.custom_jvm_args')}</label>
                            <input
                                type="text"
                                value={localSettings.custom_java_args}
                                onChange={(e) => setLocalSettings({ ...localSettings, custom_java_args: e.target.value })}
                                placeholder="-XX:+UseG1GC -Xmx4G ..."
                                className="w-full bg-white/5 border border-white/10 rounded-lg px-4 py-3 text-sm focus:outline-none focus:border-sky-400/50 focus:bg-white/[0.08] transition-all"
                            />
                        </div>
                    </section>

                    {/* Behavior section */}
                    <section className="space-y-6 pt-4 border-t border-white/5">
                        <div className="flex items-center gap-2 mb-2">
                            <Bot className="w-4 h-4 text-sky-400/60" />
                            <h3 className="text-sm font-bold uppercase tracking-wider text-white/50">{t('settings.behavior_social')}</h3>
                        </div>

                        <div className="space-y-4">
                            <label className="flex items-center justify-between p-4 bg-white/[0.02] border border-white/5 rounded-xl hover:bg-white/[0.04] transition-colors cursor-pointer group">
                                <div className="space-y-0.5">
                                    <span className="font-semibold text-white/90">{t('settings.discord_rpc')}</span>
                                    <p className="text-xs text-white/40">{t('settings.discord_rpc_desc')}</p>
                                </div>
                                <input
                                    type="checkbox"
                                    checked={localSettings.discord_rpc_enabled}
                                    onChange={(e) => setLocalSettings({ ...localSettings, discord_rpc_enabled: e.target.checked })}
                                    className="w-5 h-5 accent-sky-400 rounded border-white/10"
                                />
                            </label>

                            <label className="flex items-center justify-between p-4 bg-white/[0.02] border border-white/5 rounded-xl hover:bg-white/[0.04] transition-colors cursor-pointer group">
                                <div className="space-y-0.5">
                                    <span className="font-semibold text-white/90">{t('settings.minimize_to_tray')}</span>
                                    <p className="text-xs text-white/40">{t('settings.minimize_to_tray_desc')}</p>
                                </div>
                                <input
                                    type="checkbox"
                                    checked={localSettings.minimize_to_tray}
                                    onChange={(e) => setLocalSettings({ ...localSettings, minimize_to_tray: e.target.checked })}
                                    className="w-5 h-5 accent-sky-400 rounded border-white/10"
                                />
                            </label>

                            <label className="flex items-center justify-between p-4 bg-white/[0.02] border border-white/5 rounded-xl hover:bg-white/[0.04] transition-colors cursor-pointer group">
                                <div className="space-y-0.5">
                                    <span className="font-semibold text-white/90">{t('settings.close_on_launch')}</span>
                                    <p className="text-xs text-white/40">{t('settings.close_on_launch_desc')}</p>
                                </div>
                                <input
                                    type="checkbox"
                                    checked={localSettings.close_on_launch}
                                    onChange={(e) => setLocalSettings({ ...localSettings, close_on_launch: e.target.checked })}
                                    className="w-5 h-5 accent-sky-400 rounded border-white/10"
                                />
                            </label>
                        </div>
                    </section>

                </div>

                {/* Footer */}
                <div className="px-8 py-6 border-t border-white/5 bg-white/[0.02] flex justify-end gap-3">
                    <button
                        onClick={onClose}
                        className="px-6 py-2.5 text-sm font-semibold text-white/60 hover:text-white transition-colors"
                    >
                        {t('common.cancel')}
                    </button>
                    <button
                        onClick={handleSave}
                        disabled={loading}
                        className="px-8 py-2.5 bg-gradient-to-r from-sky-500 to-blue-600 rounded-lg text-white font-bold text-sm hover:scale-105 active:scale-95 transition-all flex items-center gap-2 disabled:opacity-50"
                    >
                        <Save className="w-4 h-4" />
                        {t('settings.save_changes')}
                    </button>
                </div>
            </div>
        </div>
    );
};
