import { useState, useEffect } from 'react';
import { Globe, Layers, ArrowLeft, Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import ModBrowser from './ModBrowser';
import ModLibrary from './ModLibrary';
import { ProfileManager } from './ProfileManager';
import { useModStore } from '../store/useModStore';

interface ModsPageProps {
    onBack: () => void;
}

export default function ModsPage({ onBack }: ModsPageProps) {
    const { t } = useTranslation();
    const [activeTab, setActiveTab] = useState<'browse' | 'library' | 'modpacks'>('library');

    const {
        installedMods,
        activeProfile,
        fetchInstalledMods,
        fetchActiveProfile,
        checkUpdates
    } = useModStore();

    // Initial fetch
    useEffect(() => {
        fetchInstalledMods();
        fetchActiveProfile();
    }, []);

    // Check for updates once
    useEffect(() => {
        if (installedMods.length > 0) {
            checkUpdates();
        }
    }, [installedMods.length === 0]);

    return (
        <div className="flex flex-col h-screen bg-[#0c0f16] text-white">
            <div className="h-16 border-b border-white/5 bg-[#141822] px-6 flex items-center justify-between shadow-xl z-30">
                <div className="flex items-center gap-4">
                    <button
                        onClick={onBack}
                        className="p-2 -ml-2 rounded-lg text-white/50 hover:text-white hover:bg-white/5 transition-colors cursor-pointer"
                        title={t('common.close')}
                    >
                        <ArrowLeft />
                    </button>
                    <h1 className="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-sky-400 to-blue-500">
                        {t('mods.title')}
                    </h1>
                    <div className="h-6 w-px bg-white/10 mx-2" />
                    <nav className="flex gap-2">
                        <button
                            onClick={() => setActiveTab('library')}
                            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all flex items-center gap-2 cursor-pointer ${activeTab === 'library'
                                ? 'bg-white/10 text-white shadow-inner'
                                : 'text-white/50 hover:text-white hover:bg-white/5'
                                }`}
                        >
                            <Layers size={18} /> {t('mods.library')}
                            <span className="ml-1 bg-black/20 px-2 py-0.5 rounded-full text-xs">
                                {installedMods.length}
                            </span>
                        </button>
                        <button
                            onClick={() => setActiveTab('browse')}
                            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all flex items-center gap-2 cursor-pointer ${activeTab === 'browse'
                                ? 'bg-white/10 text-white shadow-inner'
                                : 'text-white/50 hover:text-white hover:bg-white/5'
                                }`}
                        >
                            <Globe size={18} /> {t('mods.browse')}
                        </button>
                        <button
                            onClick={() => setActiveTab('modpacks')}
                            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all flex items-center gap-2 cursor-pointer ${activeTab === 'modpacks'
                                ? 'bg-white/10 text-white shadow-inner'
                                : 'text-white/50 hover:text-white hover:bg-white/5'
                                }`}
                        >
                            <Package size={18} /> {t('mods.profiles')}
                        </button>
                    </nav>
                </div>

                <div className="flex items-center gap-3">
                    <div className="flex flex-col items-end mr-2">
                        <span className="text-[10px] uppercase font-bold text-white/20 tracking-widest">{t('mods.active_profile') || 'Perfil Ativo'}</span>
                        <span className="text-xs font-bold text-sky-400 flex items-center gap-1.5">
                            <div className="w-1.5 h-1.5 rounded-full bg-sky-500 animate-pulse shadow-[0_0_8px_rgba(56,189,248,0.5)]" />
                            {activeProfile}
                        </span>
                    </div>
                </div>
            </div>

            <div className="flex-1 relative overflow-hidden">
                {activeTab === 'browse' ? (
                    <ModBrowser />
                ) : activeTab === 'library' ? (
                    <ModLibrary />
                ) : (
                    <ProfileManager />
                )}
            </div>
        </div>
    );
}
