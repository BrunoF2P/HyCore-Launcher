import { useModStore } from '../store/useModStore';
import ModCard from './ModCard';
import { Package } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export default function ModLibrary() {
    const { t } = useTranslation();
    const {
        installedMods: mods,
        toggleMod: onToggle,
        removeMod: onRemove,
        updateMod: onUpdate,
        loading: isLoading
    } = useModStore();

    const handleToggle = (mod: any, enabled: boolean) => {
        onToggle(mod.id, enabled).catch(err => alert(t('mods.toggle_failed') + ': ' + err));
    };

    const handleRemove = (mod: any) => {
        if (!confirm(t('mods.confirm_uninstall', { name: mod.name }))) return;
        onRemove(mod.id).catch(err => alert(t('mods.remove_failed') + ': ' + err));
    };

    const handleUpdate = (mod: any) => {
        onUpdate(mod).catch(err => alert(t('mods.install_failed') + ': ' + err));
    };

    if (mods.length === 0 && !isLoading) {
        return (
            <div className="flex flex-col items-center justify-center h-full text-white/30 p-8">
                <Package className="w-16 h-16 mb-4 opacity-50" />
                <h2 className="text-xl font-bold mb-2">{t('mods.no_installed')}</h2>
                <p className="text-center max-w-sm">
                    {t('mods.no_installed_desc')}
                </p>
            </div>
        );
    }

    return (
        <div className="flex-1 overflow-y-auto p-6 scrollbar-thin scrollbar-thumb-white/10 scrollbar-track-transparent">
            <div className="max-w-6xl mx-auto">
                <h2 className="text-white/50 uppercase text-xs font-bold tracking-wider mb-4 pl-1">
                    {t('mods.installed')} ({mods.length})
                </h2>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                    {mods.map(mod => (
                        <ModCard
                            key={mod.id}
                            mod={mod}
                            isInstalled={true}
                            onToggle={handleToggle}
                            onRemove={handleRemove}
                            onUpdate={handleUpdate}
                        />
                    ))}
                </div>
            </div>
        </div>
    );
}
