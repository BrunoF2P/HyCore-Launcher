import { useTranslation } from "react-i18next";
import { X, Cpu, Save, Settings, Bot } from "lucide-react";
import { useEffect, useState } from "react";
import { useSettingsStore } from "../store/useSettingsStore";

interface SettingsOverlayProps {
    onClose: () => void;
}

export const SettingsOverlay = ({ onClose }: SettingsOverlayProps) => {
    const { } = useTranslation();
    const { settings, loadSettings, updateSettings, loading } = useSettingsStore();
    const [localSettings, setLocalSettings] = useState(settings);

    useEffect(() => {
        loadSettings();
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

            <div className="relative w-full max-w-2xl bg-[#11141d] border border-white/5 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[80vh] animate-in zoom-in-95 fade-in duration-300">
                {/* Header */}
                <div className="flex items-center justify-between px-8 py-6 border-b border-white/5 bg-white/[0.02]">
                    <div className="flex items-center gap-3">
                        <div className="p-2 bg-hyamber/10 rounded-lg">
                            <Settings className="w-5 h-5 text-hyamber" />
                        </div>
                        <div>
                            <h2 className="text-xl font-bold tracking-tight">Configurações do Launcher</h2>
                            <p className="text-xs text-white/40 uppercase tracking-widest font-medium mt-0.5">Hycore Management</p>
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
                            <Cpu className="w-4 h-4 text-hyamber/60" />
                            <h3 className="text-sm font-bold uppercase tracking-wider text-white/50">Performance Java</h3>
                        </div>

                        <div className="grid grid-cols-2 gap-6">
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-white/70 flex justify-between">
                                    <span>RAM Mínima (GB)</span>
                                    <span className="text-hyamber">{localSettings.ram_min_gb} GB</span>
                                </label>
                                <input
                                    type="range"
                                    min="1"
                                    max="16"
                                    step="1"
                                    value={localSettings.ram_min_gb}
                                    onChange={(e) => setLocalSettings({ ...localSettings, ram_min_gb: parseInt(e.target.value) })}
                                    className="w-full accent-hyamber h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer"
                                />
                            </div>
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-white/70 flex justify-between">
                                    <span>RAM Máxima (GB)</span>
                                    <span className="text-hyamber">{localSettings.ram_max_gb} GB</span>
                                </label>
                                <input
                                    type="range"
                                    min="2"
                                    max="32"
                                    step="1"
                                    value={localSettings.ram_max_gb}
                                    onChange={(e) => setLocalSettings({ ...localSettings, ram_max_gb: parseInt(e.target.value) })}
                                    className="w-full accent-hyamber h-1.5 bg-white/10 rounded-lg appearance-none cursor-pointer"
                                />
                            </div>
                        </div>

                        <div className="space-y-2">
                            <label className="text-sm font-medium text-white/70">Argumentos da JVM (Custom)</label>
                            <input
                                type="text"
                                value={localSettings.custom_java_args}
                                onChange={(e) => setLocalSettings({ ...localSettings, custom_java_args: e.target.value })}
                                placeholder="-XX:+UseG1GC -Xmx4G ..."
                                className="w-full bg-white/5 border border-white/10 rounded-lg px-4 py-3 text-sm focus:outline-none focus:border-hyamber/50 focus:bg-white/[0.08] transition-all"
                            />
                        </div>
                    </section>

                    {/* Behavior section */}
                    <section className="space-y-6 pt-4 border-t border-white/5">
                        <div className="flex items-center gap-2 mb-2">
                            <Bot className="w-4 h-4 text-hyamber/60" />
                            <h3 className="text-sm font-bold uppercase tracking-wider text-white/50">Comportamento e Social</h3>
                        </div>

                        <div className="space-y-4">
                            <label className="flex items-center justify-between p-4 bg-white/[0.02] border border-white/5 rounded-xl hover:bg-white/[0.04] transition-colors cursor-pointer group">
                                <div className="space-y-0.5">
                                    <span className="font-semibold text-white/90">Discord Rich Presence</span>
                                    <p className="text-xs text-white/40">Exibir status do jogo no seu perfil do Discord.</p>
                                </div>
                                <input
                                    type="checkbox"
                                    checked={localSettings.discord_rpc_enabled}
                                    onChange={(e) => setLocalSettings({ ...localSettings, discord_rpc_enabled: e.target.checked })}
                                    className="w-5 h-5 accent-hyamber rounded border-white/10"
                                />
                            </label>

                            <label className="flex items-center justify-between p-4 bg-white/[0.02] border border-white/5 rounded-xl hover:bg-white/[0.04] transition-colors cursor-pointer group">
                                <div className="space-y-0.5">
                                    <span className="font-semibold text-white/90">Minimizar para a Bandeja</span>
                                    <p className="text-xs text-white/40">Ao fechar o launcher, ele continuará rodando na tray.</p>
                                </div>
                                <input
                                    type="checkbox"
                                    checked={localSettings.minimize_to_tray}
                                    onChange={(e) => setLocalSettings({ ...localSettings, minimize_to_tray: e.target.checked })}
                                    className="w-5 h-5 accent-hyamber rounded border-white/10"
                                />
                            </label>

                            <label className="flex items-center justify-between p-4 bg-white/[0.02] border border-white/5 rounded-xl hover:bg-white/[0.04] transition-colors cursor-pointer group">
                                <div className="space-y-0.5">
                                    <span className="font-semibold text-white/90">Fechar ao Iniciar Jogo</span>
                                    <p className="text-xs text-white/40">O launcher será encerrado automaticamente ao abrir o jogo.</p>
                                </div>
                                <input
                                    type="checkbox"
                                    checked={localSettings.close_on_launch}
                                    onChange={(e) => setLocalSettings({ ...localSettings, close_on_launch: e.target.checked })}
                                    className="w-5 h-5 accent-hyamber rounded border-white/10"
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
                        Cancelar
                    </button>
                    <button
                        onClick={handleSave}
                        disabled={loading}
                        className="px-8 py-2.5 bg-hyamber rounded-lg text-black font-bold text-sm hover:scale-105 active:scale-95 transition-all flex items-center gap-2 shadow-lg shadow-hyamber/20 disabled:opacity-50"
                    >
                        <Save className="w-4 h-4" />
                        Salvar Alterações
                    </button>
                </div>
            </div>
        </div>
    );
};
