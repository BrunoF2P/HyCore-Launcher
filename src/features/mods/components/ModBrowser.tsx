import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Search, Filter, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import ModCard from './ModCard';
import { SearchResult, CurseForgeMod, SearchModsParams } from '../types';
import { CategorySidebar } from './CategorySidebar';
import { ModDetailOverlay } from './ModDetailOverlay';
import { useModStore } from '../store/useModStore';

export default function ModBrowser() {
    const { installedMods, installingIds, installMod } = useModStore();
    const { t } = useTranslation();

    const onInstallRequest = (mod: any) => {
        installMod(mod).catch(err => alert(`${t('mods.install_failed')}: ${err}`));
    };
    const [query, setQuery] = useState('');
    const [mods, setMods] = useState<CurseForgeMod[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [page, setPage] = useState(0);
    const [total, setTotal] = useState(0);

    const [showFilters, setShowFilters] = useState(false);
    const [sortField, setSortField] = useState(2); // Popularity default
    const [sortOrder, setSortOrder] = useState('desc');
    const [categoryId, setCategoryId] = useState<number | undefined>();

    const [selectedMod, setSelectedMod] = useState<CurseForgeMod | null>(null);

    const search = useCallback(async (reset = false) => {
        if (loading && !reset) return;

        setLoading(true);
        setError(null);

        const currentIndex = reset ? 0 : page * 20;

        const params: SearchModsParams = {
            query: query.trim() || undefined,
            pageSize: 20,
            index: currentIndex,
            sortField: sortField,
            sortOrder: sortOrder,
            categoryId: categoryId
        };

        try {
            const result = await invoke<SearchResult>('search_mods_cf', { params });

            if (reset) {
                setMods(result.mods);
                setPage(1);
            } else {
                setMods(prev => [...prev, ...result.mods]);
                setPage(prev => prev + 1);
            }
            setTotal(result.totalCount);

        } catch (err: any) {
            setError(err.toString());
        } finally {
            setLoading(false);
        }
    }, [query, page, loading, sortField, sortOrder, categoryId]);

    useEffect(() => {
        const handler = setTimeout(() => {
            search(true);
        }, 300); // Shorter debounce for general search
        return () => clearTimeout(handler);
    }, [query, categoryId, sortField, sortOrder]);

    const isInstalled = (cfId: number) => {
        return installedMods.some(m => m.curseForgeId === cfId);
    };

    const SORT_OPTIONS = [
        { id: 1, name: t('mods.sort.featured') },
        { id: 2, name: t('mods.sort.popularity') },
        { id: 3, name: t('mods.sort.last_updated') },
        { id: 6, name: t('mods.sort.total_downloads') },
        { id: 4, name: t('mods.sort.name') },
        { id: 5, name: t('mods.sort.author') },
    ];

    return (
        <div className="flex h-full bg-[#0c0f16] overflow-hidden">
            <CategorySidebar
                onSelectCategory={setCategoryId}
                activeCategoryId={categoryId}
            />

            <div className="flex-1 flex flex-col min-w-0">
                <div className="p-6 border-b border-white/5 bg-[#141822]/50 backdrop-blur-md sticky top-0 z-20">
                    <div className="flex flex-col gap-4 max-w-5xl mx-auto">
                        <div className="flex gap-4">
                            <div className="relative flex-1">
                                <Search className="absolute left-4 top-1/2 -translate-y-1/2 text-white/30" />
                                <input
                                    type="text"
                                    placeholder={t('mods.search_placeholder')}
                                    value={query}
                                    onChange={(e) => setQuery(e.target.value)}
                                    className="w-full bg-black/20 border border-white/10 rounded-xl pl-10 pr-4 py-3 text-white placeholder-white/30 focus:outline-none focus:border-sky-500/50 transition-colors"
                                />
                            </div>
                            <button
                                onClick={() => setShowFilters(!showFilters)}
                                className={`px-4 py-3 border rounded-xl transition-colors flex items-center gap-2 cursor-pointer ${showFilters
                                    ? 'bg-sky-500/20 border-sky-500/30 text-sky-400'
                                    : 'bg-white/5 border-white/10 text-white/70 hover:bg-white/10 hover:text-white'
                                    }`}
                            >
                                <Filter size={18} /> {t('mods.filters')}
                            </button>
                        </div>

                        {showFilters && (
                            <div className="bg-black/20 border border-white/5 rounded-xl p-4 animate-in fade-in slide-in-from-top-2">
                                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div>
                                        <label className="text-xs uppercase font-bold text-white/40 mb-2 block">{t('mods.sort.by')}</label>
                                        <div className="flex flex-wrap gap-2">
                                            {SORT_OPTIONS.map(opt => (
                                                <button
                                                    key={opt.id}
                                                    onClick={() => setSortField(opt.id)}
                                                    className={`px-3 py-1.5 rounded-lg text-sm transition-colors cursor-pointer ${sortField === opt.id
                                                        ? 'bg-sky-500 text-white shadow-lg shadow-sky-500/20'
                                                        : 'bg-white/5 text-white/50 hover:text-white hover:bg-white/10'
                                                        }`}
                                                >
                                                    {opt.name}
                                                </button>
                                            ))}
                                        </div>
                                    </div>
                                    <div>
                                        <label className="text-xs uppercase font-bold text-white/40 mb-2 block">{t('mods.sort.order')}</label>
                                        <div className="flex bg-white/5 rounded-lg p-1 w-fit">
                                            <button
                                                onClick={() => setSortOrder('desc')}
                                                className={`px-4 py-1.5 rounded-md text-sm transition-all cursor-pointer ${sortOrder === 'desc'
                                                    ? 'bg-sky-500 text-white shadow-md'
                                                    : 'text-white/50 hover:text-white'
                                                    }`}
                                            >
                                                {t('mods.sort.desc')}
                                            </button>
                                            <button
                                                onClick={() => setSortOrder('asc')}
                                                className={`px-4 py-1.5 rounded-md text-sm transition-all cursor-pointer ${sortOrder === 'asc'
                                                    ? 'bg-sky-500 text-white shadow-md'
                                                    : 'text-white/50 hover:text-white'
                                                    }`}
                                            >
                                                {t('mods.sort.asc')}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>
                </div>

                <div className="flex-1 overflow-y-auto p-6 scrollbar-thin scrollbar-thumb-white/10 scrollbar-track-transparent">
                    <div className="max-w-5xl mx-auto">
                        {error && (
                            <div className="bg-red-500/10 border border-red-500/20 text-red-500 p-4 rounded-xl mb-6 text-center">
                                {error}
                            </div>
                        )}

                        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-6">
                            {mods.map(mod => (
                                <ModCard
                                    key={mod.id}
                                    mod={mod}
                                    isInstalled={isInstalled(mod.id)}
                                    isLoading={installingIds.includes(mod.id)}
                                    onInstall={() => onInstallRequest(mod)}
                                    onClick={() => setSelectedMod(mod)}
                                />
                            ))}
                        </div>

                        {loading && (
                            <div className="py-12 flex justify-center">
                                <Loader2 className="animate-spin text-sky-500 text-3xl" />
                            </div>
                        )}

                        {!loading && mods.length === 0 && !error && (
                            <div className="text-center py-20 text-white/30">
                                <p className="text-xl">{t('mods.no_results')}</p>
                                <p className="text-sm mt-2">{t('mods.no_results_desc')}</p>
                            </div>
                        )}

                        {!loading && mods.length < total && mods.length > 0 && (
                            <div className="py-8 text-center">
                                <button
                                    onClick={() => search(false)}
                                    className="px-6 py-2 bg-white/5 hover:bg-white/10 border border-white/10 rounded-full text-white/70 transition-colors text-sm cursor-pointer"
                                >
                                    {t('mods.load_more')}
                                </button>
                            </div>
                        )}
                    </div>
                </div>
            </div>

            {selectedMod && (
                <ModDetailOverlay
                    mod={selectedMod}
                    onClose={() => setSelectedMod(null)}
                />
            )}
        </div>
    );
}
