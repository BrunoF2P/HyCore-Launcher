import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Globe, Layers, ArrowLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import ModBrowser from './ModBrowser';
import ModLibrary from './ModLibrary';
import { CurseForgeMod, InstalledMod } from './types';

interface ModsPageProps {
    onBack: () => void;
}

export default function ModsPage({ onBack }: ModsPageProps) {
    const { t } = useTranslation();
    const [activeTab, setActiveTab] = useState<'browse' | 'library'>('library');
    const [installedMods, setInstalledMods] = useState<InstalledMod[]>([]);
    const [refreshTrigger, setRefreshTrigger] = useState(0);
    const [installingIds, setInstallingIds] = useState<number[]>([]);

    // Fetch installed mods
    useEffect(() => {
        invoke<InstalledMod[]>('get_installed_mods')
            .then(setInstalledMods)
            .catch(console.error);
    }, [refreshTrigger]);

    const handleInstall = async (mod: CurseForgeMod) => {
        setInstallingIds(prev => [...prev, mod.id]);
        try {
            await invoke('install_mod_cf', { modId: mod.id, fileId: null });
            // Success
            setRefreshTrigger(prev => prev + 1);
            // Optional: Switch to library or show notification
        } catch (error) {
            console.error('Install failed:', error);
            alert(`Failed to install mod: ${error}`);
        } finally {
            setInstallingIds(prev => prev.filter(id => id !== mod.id));
        }
    };

    const handleRemove = async (mod: InstalledMod) => {
        if (!confirm(t('mods.confirm_uninstall', { name: mod.name }))) return;

        try {
            await invoke('remove_mod', { modId: mod.id });
            setRefreshTrigger(prev => prev + 1);
        } catch (error) {
            console.error('Remove failed:', error);
            alert(`Failed to remove mod: ${error}`);
        }
    };

    const handleToggle = async (mod: InstalledMod, enabled: boolean) => {
        try {
            await invoke('toggle_mod', { modId: mod.id, enabled });
            setRefreshTrigger(prev => prev + 1);
        } catch (error) {
            console.error('Toggle failed:', error);
            alert(`Failed to toggle mod: ${error}`);
        }
    }

    return (
        <div className="flex flex-col h-screen bg-[#121418] text-white">
            {/* Top Navigation Bar (Shared with other pages style usually) */}
            <div className="h-16 border-b border-white/5 bg-[#1a1d23] px-6 flex items-center justify-between shadow-xl z-30">
                <div className="flex items-center gap-4">
                    <button
                        onClick={onBack}
                        className="p-2 -ml-2 rounded-lg text-white/50 hover:text-white hover:bg-white/5 transition-colors"
                        title={t('common.close')}
                    >
                        <ArrowLeft />
                    </button>
                    <h1 className="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-emerald-400 to-cyan-500">
                        {t('mods.title')}
                    </h1>
                    <div className="h-6 w-px bg-white/10 mx-2" />
                    <nav className="flex gap-2">
                        <button
                            onClick={() => setActiveTab('library')}
                            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all flex items-center gap-2 ${activeTab === 'library'
                                ? 'bg-white/10 text-white shadow-inner'
                                : 'text-white/50 hover:text-white hover:bg-white/5'
                                }`}
                        >
                            <Layers /> {t('mods.library')}
                            <span className="ml-1 bg-black/20 px-2 py-0.5 rounded-full text-xs">
                                {installedMods.length}
                            </span>
                        </button>
                        <button
                            onClick={() => setActiveTab('browse')}
                            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all flex items-center gap-2 ${activeTab === 'browse'
                                ? 'bg-white/10 text-white shadow-inner'
                                : 'text-white/50 hover:text-white hover:bg-white/5'
                                }`}
                        >
                            <Globe /> {t('mods.browse')}
                        </button>
                    </nav>
                </div>
            </div>

            {/* Content Area */}
            <div className="flex-1 relative overflow-hidden">
                {activeTab === 'browse' ? (
                    <ModBrowser
                        installedMods={installedMods}
                        onInstallRequest={handleInstall}
                        installingIds={installingIds}
                    />
                ) : (
                    <ModLibrary
                        mods={installedMods}
                        onToggle={handleToggle}
                        onRemove={handleRemove}
                        isLoading={false}
                    />
                )}
            </div>
        </div>
    );
}
